//! SFU 引擎：单线程驱动所有房间的所有 Peer（str0m Sans-IO）。
//!
//! 结构改编自 str0m 官方 chat.rs 示例，做三处裁剪：
//! 1. 纯音频——删除视频 keyframe 相关逻辑；
//! 2. 房间隔离——MediaData 只在同一 `room` 的 Peer 间转发；
//! 3. HTTP 协商——订阅侧 offer 不走 data channel，由 HTTP `subscribe` 拉取、`answer` 回填。
//!
//! 房间没有实体：第一个 publish 即创建，最后一个 Peer 离开即消失。

use std::collections::{HashMap, VecDeque};
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use str0m::change::{SdpAnswer, SdpOffer, SdpPendingOffer};
use str0m::media::{Direction, MediaData, MediaKind, Mid};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc};

/// 新 Peer：publish 时在 HTTP 线程完成 accept_offer，Rtc 实例扔进引擎驱动。
pub struct NewPeer {
    pub rtc: Rtc,
    pub room: String,
    pub user: String,
}

pub enum Cmd {
    Subscribe {
        room: String,
        user: String,
        reply: std::sync::mpsc::SyncSender<Option<SdpOffer>>,
    },
    Answer {
        room: String,
        user: String,
        answer: SdpAnswer,
        reply: std::sync::mpsc::SyncSender<bool>,
    },
    Trickle {
        room: String,
        user: String,
        candidate: Candidate,
        reply: std::sync::mpsc::SyncSender<bool>,
    },
    /// 房间花名册：[{id: "数字 peer id", name: 昵称}]，id 与浏览器 stream.id 对应
    Roster {
        room: String,
        reply: std::sync::mpsc::SyncSender<Value>,
    },
    /// 聊天/弹幕消息入房缓冲，返回分配的消息 id
    ChatSend {
        room: String,
        user: String,
        text: String,
        reply: std::sync::mpsc::SyncSender<u64>,
    },
    /// 增量拉取房间消息（id > since），房间已空则连缓冲一起清掉
    ChatFetch {
        room: String,
        since: u64,
        reply: std::sync::mpsc::SyncSender<Value>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PeerId(u64);

/// 每房聊天/弹幕缓冲：环形 200 条
#[derive(Debug, Default)]
struct ChatBuf {
    next_id: u64,
    msgs: VecDeque<Value>,
}

#[derive(Debug)]
struct TrackIn {
    origin: PeerId,
    mid: Mid,
    kind: MediaKind,
}

#[derive(Debug)]
struct TrackInEntry {
    id: Arc<TrackIn>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackOutState {
    ToOpen,
    Negotiating(Mid),
    Open(Mid),
}

#[derive(Debug)]
struct TrackOut {
    track_in: std::sync::Weak<TrackIn>,
    state: TrackOutState,
}

impl TrackOut {
    fn mid(&self) -> Option<Mid> {
        match self.state {
            TrackOutState::ToOpen => None,
            TrackOutState::Negotiating(m) | TrackOutState::Open(m) => Some(m),
        }
    }
}

struct Peer {
    id: PeerId,
    room: String,
    user: String,
    /// 加入时间（Unix 毫秒），成员浮层的「加入时间」用
    joined_ms: u64,
    rtc: Rtc,
    pending: Option<SdpPendingOffer>,
    tracks_in: Vec<TrackInEntry>,
    tracks_out: Vec<TrackOut>,
}

static NEXT_PEER_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Peer {
    fn new(np: NewPeer) -> Self {
        let joined_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            id: PeerId(NEXT_PEER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)),
            room: np.room,
            user: np.user,
            joined_ms,
            rtc: np.rtc,
            pending: None,
            tracks_in: vec![],
            tracks_out: vec![],
        }
    }

    fn accepts(&self, input: &Input) -> bool {
        self.rtc.accepts(input)
    }

    fn handle_input(&mut self, input: Input) {
        if !self.rtc.is_alive() {
            return;
        }
        if let Err(e) = self.rtc.handle_input(input) {
            tracing::warn!(peer = %self.user, room = %self.room, "peer input error, disconnecting: {e:?}");
            self.rtc.disconnect();
        }
    }

    fn poll_output(&mut self, socket: &UdpSocket) -> Propagated {
        if !self.rtc.is_alive() {
            return Propagated::Noop;
        }

        match self.rtc.poll_output() {
            Ok(output) => self.handle_output(output, socket),
            Err(e) => {
                tracing::warn!(peer = %self.user, "poll_output failed: {e:?}");
                self.rtc.disconnect();
                Propagated::Noop
            }
        }
    }

    fn handle_output(&mut self, output: Output, socket: &UdpSocket) -> Propagated {
        match output {
            Output::Transmit(transmit) => {
                let _ = socket.send_to(&transmit.contents, transmit.destination);
                Propagated::Noop
            }
            Output::Timeout(t) => Propagated::Timeout(t),
            Output::Event(e) => match e {
                Event::IceConnectionStateChange(v) => {
                    if v == IceConnectionState::Disconnected {
                        tracing::info!(room = %self.room, peer = %self.user, "ICE disconnected");
                        self.rtc.disconnect();
                    }
                    if v == IceConnectionState::Connected {
                        tracing::info!(room = %self.room, peer = %self.user, "ICE connected ✓");
                    }
                    Propagated::Noop
                }
                Event::MediaAdded(m) => {
                    // 自己 publish 的音频轨上线；开关麦会换 SSRC 触发重复 MediaAdded，
                    // 按 mid 去重——同一轨反复通知不能再生成新的下行 m-line
                    if self.tracks_in.iter().any(|e| e.id.mid == m.mid) {
                        tracing::debug!(room = %self.room, peer = %self.user, mid = ?m.mid, "media re-added, ignored");
                        return Propagated::Noop;
                    }
                    let entry = TrackInEntry {
                        id: Arc::new(TrackIn { origin: self.id, mid: m.mid, kind: m.kind }),
                    };
                    self.tracks_in.push(entry);
                    tracing::info!(room = %self.room, peer = %self.user, mid = ?m.mid, "media added");
                    Propagated::Noop
                }
                Event::MediaData(data) => Propagated::MediaData(self.room.clone(), self.id, data),
                _ => Propagated::Noop,
            },
        }
    }

    /// 把来源为 origin 的下行音频写入本 Peer 的对应轨
    fn handle_media_data_out(&mut self, origin: PeerId, data: &MediaData) {
        let Some(mid) = self
            .tracks_out
            .iter()
            .find(|o| {
                o.track_in
                    .upgrade()
                    .filter(|i| i.origin == origin && i.mid == data.mid)
                    .is_some()
            })
            .and_then(|o| o.mid())
        else {
            return;
        };

        let Some(writer) = self.rtc.writer(mid) else {
            return;
        };
        let Some(pt) = writer.match_params(data.params) else {
            return;
        };
        let _ = writer.write(pt, data.network_time, data.time, data.data.clone());
    }
}

enum Propagated {
    MediaData(String, PeerId, MediaData),
    Timeout(Instant),
    Noop,
}

/// 订阅：把同房间其他 Peer 的轨加进 ToOpen，并立刻协商出 offer（若有新轨）。
fn peer_subscribe(me: &mut Peer, others: Vec<&Peer>) -> Option<SdpOffer> {
    for other in &others {
        for t in &other.tracks_in {
            let known = me.tracks_out.iter().any(|o| {
                o.track_in
                    .upgrade()
                    .map(|i| i.origin == other.id && i.mid == t.id.mid)
                    .unwrap_or(false)
            });
            if !known {
                me.tracks_out.push(TrackOut {
                    track_in: Arc::downgrade(&t.id),
                    state: TrackOutState::ToOpen,
                });
            }
        }
    }

    if !me.tracks_out.iter().any(|o| o.state == TrackOutState::ToOpen) {
        return None; // 没有新轨
    }

    let mut change = me.rtc.sdp_api();
    for track in &mut me.tracks_out {
        if let TrackOutState::ToOpen = track.state {
            if let Some(t_in) = track.track_in.upgrade() {
                let mid = change.add_media(
                    t_in.kind,
                    Direction::SendOnly,
                    Some(t_in.origin.0.to_string()),
                    None,
                    None,
                );
                track.state = TrackOutState::Negotiating(mid);
            }
        }
    }

    let (offer, pending) = change.apply()?;
    me.pending = Some(pending);
    Some(offer)
}

fn peer_accept_answer(me: &mut Peer, answer: SdpAnswer) -> bool {
    let Some(pending) = me.pending.take() else {
        return false;
    };
    if me.rtc.sdp_api().accept_answer(pending, answer).is_err() {
        tracing::warn!(peer = %me.user, "accept_answer failed");
        return false;
    }
    for track in &mut me.tracks_out {
        if let TrackOutState::Negotiating(m) = track.state {
            track.state = TrackOutState::Open(m);
        }
    }
    true
}

fn handle_cmd(
    owners: &mut HashMap<String, String>,
    chats: &mut HashMap<String, ChatBuf>,
    peers: &mut [Peer],
    cmd: Cmd,
) {
    match cmd {
        Cmd::Subscribe { room, user, reply } => {
            let offer = match peers.iter().position(|p| p.room == room && p.user == user) {
                Some(idx) => {
                    let (before, after) = peers.split_at_mut(idx);
                    let (me, rest) = after.split_first_mut().expect("at least self");
                    let others: Vec<&Peer> =
                        before.iter().chain(rest.iter()).filter(|p| p.room == room).collect();
                    peer_subscribe(me, others)
                }
                None => None,
            };
            let _ = reply.send(offer);
        }
        Cmd::Answer { room, user, answer, reply } => {
            let ok = peers
                .iter_mut()
                .find(|p| p.room == room && p.user == user)
                .map(|p| peer_accept_answer(p, answer))
                .unwrap_or(false);
            let _ = reply.send(ok);
        }
        Cmd::Trickle { room, user, candidate, reply } => {
            let ok = peers
                .iter_mut()
                .find(|p| p.room == room && p.user == user)
                .map(|p| {
                    p.rtc.add_remote_candidate(candidate);
                    true
                })
                .unwrap_or(false);
            let _ = reply.send(ok);
        }
        Cmd::Roster { room, reply } => {
            let owner = owners.get(&room).cloned();
            let users: Vec<Value> = peers
                .iter()
                .filter(|p| p.room == room && p.rtc.is_alive())
                .map(|p| {
                    json!({
                        "id": p.id.0.to_string(),
                        "name": p.user,
                        "owner": owner.as_deref() == Some(p.user.as_str()),
                        "joinedMs": p.joined_ms,
                    })
                })
                .collect();
            // 房间空了就清掉创建者登记，房间即「消失」，下次进入算新一轮
            if users.is_empty() && !peers.iter().any(|p| p.room == room) {
                owners.remove(&room);
                chats.remove(&room);
            }
            let _ = reply.send(Value::Array(users));
        }

        Cmd::ChatSend { room, user, text, reply } => {
            let buf = chats
                .entry(room)
                .or_insert_with(|| ChatBuf { next_id: 1, msgs: VecDeque::new() });
            let id = buf.next_id;
            buf.next_id += 1;
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            buf.msgs.push_back(json!({ "id": id, "user": user, "text": text, "ts": ts }));
            // 每房最多留 200 条，满了扔最旧的
            while buf.msgs.len() > 200 {
                buf.msgs.pop_front();
            }
            let _ = reply.send(id);
        }

        Cmd::ChatFetch { room, since, reply } => {
            // 拉消息时发现房间没人了：房间整体消失，消息缓冲一并清掉
            if !peers.iter().any(|p| p.room == room) {
                chats.remove(&room);
                owners.remove(&room);
                let _ = reply.send(json!([]));
            } else {
                let arr = chats
                    .get(&room)
                    .map(|b| {
                        let v: Vec<Value> = b
                            .msgs
                            .iter()
                            .filter(|m| m["id"].as_u64().unwrap_or(0) > since)
                            .cloned()
                            .collect();
                        Value::Array(v)
                    })
                    .unwrap_or_else(|| json!([]));
                let _ = reply.send(arr);
            }
        }
    }
}

/// 主循环：改编自 str0m chat.rs 的 run()。
/// 通道以 Arc<Mutex<Receiver>> 持有（std Receiver 不可 Clone），supervisor 重生时复用
pub fn run(
    socket: UdpSocket,
    declared_addr: SocketAddr,
    new_peers: Arc<Mutex<Receiver<NewPeer>>>,
    cmds: Arc<Mutex<Receiver<Cmd>>>,
) -> ! {
    let mut peers: Vec<Peer> = vec![];
    // 房间创建者登记：第一个进房的人 = 房主；房间空了即清除
    let mut owners: HashMap<String, String> = HashMap::new();
    // 每房聊天/弹幕缓冲：环形 200 条，房间空了整体删除
    let mut chats: HashMap<String, ChatBuf> = HashMap::new();
    let mut to_propagate: VecDeque<Propagated> = VecDeque::new();
    let mut buf = vec![0u8; 2000];
    let mut stats_at = Instant::now();
    let mut media_pkts: u64 = 0u64;

    loop {
        // 清理死连接
        peers.retain(|p| p.rtc.is_alive());

        // 新 Peer 入列
        loop {
            match new_peers.lock().unwrap().try_recv() {
                Ok(np) => {
                    tracing::info!(room = %np.room, peer = %np.user, "peer joined");
                    // 顶号去重：客户端重连（点「说话」重新发布/刷新页面）时，
                    // 移除同房间同昵称的旧连接，房间内始终只有一个该用户的活跃 peer，
                    // 否则 roster 人数会随重连次数虚增
                    peers.retain(|p| !(p.room == np.room && p.user == np.user));
                    // 第一个进房的记为房主（离开再进，只要房间没空过身份不变）
                    owners
                        .entry(np.room.clone())
                        .or_insert_with(|| np.user.clone());
                    peers.push(Peer::new(np));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    tracing::error!("http layer gone, engine exits");
                    std::process::exit(1);
                }
            }
        }

        // 控制命令（subscribe / answer / trickle）
        loop {
            match cmds.lock().unwrap().try_recv() {
                Ok(cmd) => handle_cmd(&mut owners, &mut chats, &mut peers, cmd),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => std::process::exit(1),
            }
        }

        // 转发一条待传播音频：同房间、非来源本人
        if let Some(Propagated::MediaData(room, origin, data)) = to_propagate.pop_front() {
            media_pkts += 1;
            for peer in &mut peers {
                if peer.room == room && peer.id != origin {
                    peer.handle_media_data_out(origin, &data);
                }
            }
            continue;
        }

        // 轮询所有 Peer 直到 timeout
        let mut timeout = Instant::now() + Duration::from_millis(100);
        for peer in peers.iter_mut() {
            let t = poll_peer(peer, &mut to_propagate, &socket);
            timeout = timeout.min(t);
        }

        let duration = (timeout - Instant::now()).max(Duration::from_millis(1));
        socket.set_read_timeout(Some(duration)).expect("set read timeout");

        if let Some(input) = read_socket_input(&socket, declared_addr, &mut buf) {
            if let Some(peer) = peers.iter_mut().find(|p| p.accepts(&input)) {
                peer.handle_input(input);
            } else {
                tracing::debug!("no peer accepts UDP input yet");
            }
        }

        // 驱动时间
        let now = Instant::now();
        for peer in &mut peers {
            peer.handle_input(Input::Timeout(now));
        }

        // 媒体流量统计（每 10s 一条，0 包不打）
        if stats_at.elapsed() >= Duration::from_secs(10) {
            if media_pkts > 0 {
                tracing::info!(packets = media_pkts, "media forwarded in last 10s");
            }
            media_pkts = 0;
            stats_at = Instant::now();
        }
    }
}

fn poll_peer(peer: &mut Peer, queue: &mut VecDeque<Propagated>, socket: &UdpSocket) -> Instant {
    loop {
        if !peer.rtc.is_alive() {
            return Instant::now();
        }
        let propagated = peer.poll_output(socket);
        if let Propagated::Timeout(t) = propagated {
            return t;
        }
        queue.push_back(propagated);
    }
}

fn read_socket_input<'a>(socket: &UdpSocket, declared: SocketAddr, buf: &'a mut Vec<u8>) -> Option<Input<'a>> {
    buf.resize(2000, 0);
    match socket.recv_from(buf) {
        Ok((n, source)) => {
            buf.truncate(n);
            // 包探针：前 4 字节签名 —— STUN req=0x0001… DTLS=0x16fe… RTP=0x80…
            let sig = format!("{:02x}{:02x}{:02x}{:02x}", buf[0], buf[1], buf[2], buf[3]);
            let Ok(contents) = buf.as_slice().try_into() else {
                return None;
            };
            tracing::debug!(src = %source, sig = %sig, "udp in");
            Some(Input::Receive(
                Instant::now(),
                Receive {
                    proto: Protocol::Udp,
                    source,
                    // 关键：socket 绑在 0.0.0.0 上，local_addr() 会返回 0.0.0.0，
                    // str0m 用 destination 做候选配对，必须用对外宣告的公网地址
                    destination: declared,
                    contents,
                },
            ))
        }
        Err(e) => match e.kind() {
            // WOULD_BLOCK/TIMEDOUT: read_timeout 正常返回；INTERRUPTED: 系统信号
            // 打断（EINTR）；CONNECTION_REFUSED: 上一个 sendto 目标不可达的 ICMP 回
            // 报(正常瞬时)——以上全部静默重试, 决不能 panic 杀掉引擎线程
            ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted | ErrorKind::ConnectionRefused => None,
            _ => {
                tracing::warn!("udp read error (ignored): {e:?}");
                None
            }
        },
    }
}
