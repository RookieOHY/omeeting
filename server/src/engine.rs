//! SFU 引擎：单线程驱动所有房间的所有 Peer（str0m Sans-IO）。
//!
//! 结构改编自 str0m 官方 chat.rs 示例，做三处裁剪：
//! 1. 纯音频——删除视频 keyframe 相关逻辑；
//! 2. 房间隔离——MediaData 只在同一 `room` 的 Peer 间转发；
//! 3. HTTP 协商——订阅侧 offer 不走 data channel，由 HTTP `subscribe` 拉取、`answer` 回填。
//!
//! 房间没有实体：第一个 publish 即创建，最后一个 Peer 离开即消失。

use std::collections::VecDeque;
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

use crate::room::Rooms;
use crate::util::unix_millis;

/// 新 Peer：publish 时在 HTTP 线程完成 accept_offer，Rtc 实例扔进引擎驱动。
pub struct NewPeer {
    pub rtc: Rtc,
    pub room: String,
    pub user: String,
    /// 通行证：服务器签发的随机 ID，会话身份的唯一凭证（昵称只是展示名）
    pub pid: String,
}

/// HTTP 信令线程 → 引擎线程 的控制命令（见 rpc.rs 的 dispatch）。
/// 除 Subscribe/Answer/Trickle 这类媒体信令外，roster/chat 也是简单的引擎内查询。
pub enum Cmd {
    /// 拉取订阅：有未协商的新下行轨 → 返回 offer；没有 → None；pid 未知（被顶/服务器重启）→ None
    Subscribe {
        pid: String,
        reply: std::sync::mpsc::SyncSender<Option<SdpOffer>>,
    },
    /// 回填订阅协商的 answer；peer 不存在或没有 pending offer → false
    Answer {
        pid: String,
        answer: SdpAnswer,
        reply: std::sync::mpsc::SyncSender<bool>,
    },
    /// ICE candidate 补充（trickle）；peer 不存在 → false
    Trickle {
        pid: String,
        candidate: Candidate,
        reply: std::sync::mpsc::SyncSender<bool>,
    },
    /// 房间花名册：[{id: "数字 peer id", name: 昵称, owner, joinedMs}]，id 与浏览器 stream.id 对应
    Roster {
        room: String,
        reply: std::sync::mpsc::SyncSender<Value>,
    },
    /// 聊天/弹幕消息入房缓冲（按 pid 定位房间与署名），返回分配的消息 id
    ChatSend {
        pid: String,
        text: String,
        reply: std::sync::mpsc::SyncSender<Result<u64, String>>,
    },
    /// 增量拉取房间消息（id > since），房间已空则连账本一起清掉
    ChatFetch {
        room: String,
        since: u64,
        reply: std::sync::mpsc::SyncSender<Value>,
    },
}

/// 引擎内 Peer 的数字身份（自增），也同时是 str0m 下行流的 stream id
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PeerId(u64);

/// 上行轨记录：Peer 自己发布的一条音轨（Arc 共享给订阅方做引用计数）
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

/// 下行轨生命周期：ToOpen（想收）→ Negotiating（已进 offer）→ Open（answer 达成可写）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackOutState {
    ToOpen,
    Negotiating(Mid),
    Open(Mid),
}

/// 一条下行轨：弱引用指向上行轨（源头离线即自动失效，不用手动清理）
#[derive(Debug)]
struct TrackOut {
    track_in: std::sync::Weak<TrackIn>,
    state: TrackOutState,
}

impl TrackOut {
    /// 只有 Negotiating/Open 才有协商出来的 m-line id
    fn mid(&self) -> Option<Mid> {
        match self.state {
            TrackOutState::ToOpen => None,
            TrackOutState::Negotiating(m) | TrackOutState::Open(m) => Some(m),
        }
    }
}

/// 引擎里的一个参会连接：一路 publish + N 路订阅。
/// 身份字段：`room`（归属房间）、`pid`（会话通行证，唯一键）、`user`（展示昵称）。
struct Peer {
    id: PeerId,
    /// 通行证（会话唯一凭证）
    pid: String,
    room: String,
    user: String,
    /// 加入时间（Unix 毫秒），成员浮层的「加入时间」用
    joined_ms: u64,
    rtc: Rtc,
    /// 订阅侧待应答的 offer（publish 侧协商由 HTTP 线程同步完成后才进引擎，无 pending）
    pending: Option<SdpPendingOffer>,
    tracks_in: Vec<TrackInEntry>,
    tracks_out: Vec<TrackOut>,
}

static NEXT_PEER_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Peer {
    /// PeerId 由引擎全局自增，joined_ms 记在这里（HTTP 线程不掌握引擎时间线）
    fn new(np: NewPeer) -> Self {
        Self {
            id: PeerId(NEXT_PEER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)),
            pid: np.pid,
            room: np.room,
            user: np.user,
            joined_ms: unix_millis(),
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

/// Peer.poll_output 产生的待处理输出（除 Timeout 外都进待传播队列）
enum Propagated {
    /// 一包来自 origin Peer 的解码音频（等待转发给同房间其他人）
    MediaData(String, PeerId, MediaData),
    /// 该 Peer 下一个定时触发点
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

/// 回填订阅协商的 answer：把 pending offer 落地，Negotiating 的下行轨全部转 Open。
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

/// 控制命令处理：所有 Cmd 都在这里落地。媒体转发不经过此函数（主循环直推）。
fn handle_cmd(rooms: &mut Rooms, peers: &mut [Peer], cmd: Cmd) {
    match cmd {
        // 订阅：把同房间其他 Peer 的上行轨全部登记进我的下行清单，有新轨才协商 offer
        Cmd::Subscribe { pid, reply } => {
            let offer = match peers.iter().position(|p| p.pid == pid) {
                Some(idx) => {
                    let room = peers[idx].room.clone();
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
        Cmd::Answer { pid, answer, reply } => {
            let ok = peers
                .iter_mut()
                .find(|p| p.pid == pid)
                .map(|p| peer_accept_answer(p, answer))
                .unwrap_or(false);
            let _ = reply.send(ok);
        }
        Cmd::Trickle { pid, candidate, reply } => {
            let ok = peers
                .iter_mut()
                .find(|p| p.pid == pid)
                .map(|p| {
                    p.rtc.add_remote_candidate(candidate);
                    true
                })
                .unwrap_or(false);
            let _ = reply.send(ok);
        }
        Cmd::Roster { room, reply } => {
            let users: Vec<Value> = peers
                .iter()
                .filter(|p| p.room == room && p.rtc.is_alive())
                .map(|p| {
                    json!({
                        "id": p.id.0.to_string(),
                        "name": p.user,
                        "owner": rooms.is_owner(&room, &p.pid),
                        "joinedMs": p.joined_ms,
                    })
                })
                .collect();
            // 房间没人了（连非 alive 的记录都没有）→ 账本整体作废，下次进房算新一轮
            if users.is_empty() && !peers.iter().any(|p| p.room == room) {
                rooms.clear(&room);
            }
            let _ = reply.send(Value::Array(users));
        }

        Cmd::ChatSend { pid, text, reply } => {
            // 按通行证定位会话；未知 pid = 会话已失效（服务器重启/被顶），拒绝入账
            let Some(peer) = peers.iter().find(|p| p.pid == pid) else {
                let _ = reply.send(Err("session gone".into()));
                return;
            };
            let res = rooms.push_chat(&peer.room, &peer.user, &peer.pid, &text);
            if res.is_err() {
                tracing::warn!(room = %peer.room, peer = %peer.user, "chat rejected");
            }
            let _ = reply.send(res);
        }

        Cmd::ChatFetch { room, since, reply } => {
            // 拉消息时发现房间没人了：房间整体消失，账本一并清掉
            if !peers.iter().any(|p| p.room == room) {
                rooms.clear(&room);
                let _ = reply.send(json!([]));
            } else {
                let _ = reply.send(json!(rooms.fetch_chat(&room, since)));
            }
        }
    }
}

/// 主循环：改编自 str0m chat.rs 的 run()。
/// 通道以 Arc<Mutex<Receiver>> 持有（std Receiver 不可 Clone），supervisor 重生后仍可用。
///
/// 循环一圈做六件事（顺序即代码顺序）：
/// 1. 清理死连接   2. 收编新 Peer（顶号去重 + 房主登记）
/// 3. 处理控制命令 4. 把上一轮攒下的音频转发给同房间其他人
/// 5. 轮询各 Peer 的输出（含读一次 UDP 包）
/// 6. 驱动时间 & 统计
pub fn run(
    socket: UdpSocket,
    declared_addr: SocketAddr,
    new_peers: Arc<Mutex<Receiver<NewPeer>>>,
    cmds: Arc<Mutex<Receiver<Cmd>>>,
) -> ! {
    let mut peers: Vec<Peer> = vec![];
    // 房间账本（房主登记 + 聊天缓冲），与媒体转发解耦
    let mut rooms = Rooms::default();
    let mut to_propagate: VecDeque<Propagated> = VecDeque::new();
    let mut buf = vec![0u8; 2000];
    let mut stats_at = Instant::now();
    let mut media_pkts: u64 = 0u64;

    loop {
        // 1. 清理死连接（断网/被顶替换后不再存活）
        peers.retain(|p| p.rtc.is_alive());

        // 2. 新 Peer 入列（publish 可能一次并发来很多，排空为止）
        loop {
            match new_peers.lock().unwrap().try_recv() {
                Ok(np) => {
                    tracing::info!(room = %np.room, peer = %np.user, pid = %np.pid, "peer joined");
                    // 顶号：同一张通行证再发布 = 同一会话重连（如刷新页面），替换旧连接。
                    // 不同通行证 = 不同参会人，即使昵称相同也共存 —— 身份与昵称解耦。
                    peers.retain(|p| p.pid != np.pid);
                    // 第一个进房的会话记为房主（登记的是通行证，不是昵称）
                    rooms.claim_owner(&np.room, &np.pid);
                    peers.push(Peer::new(np));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // HTTP 层已死（进程退出流程），引擎无存在意义
                    std::process::exit(1);
                }
            }
        }

        // 3. 控制命令（subscribe / answer / trickle / roster / chat）
        loop {
            match cmds.lock().unwrap().try_recv() {
                Ok(cmd) => handle_cmd(&mut rooms, &mut peers, cmd),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => std::process::exit(1),
            }
        }

        // 4. 转发一条待传播音频：同房间、非来源本人
        if let Some(Propagated::MediaData(room, origin, data)) = to_propagate.pop_front() {
            media_pkts += 1;
            for peer in &mut peers {
                if peer.room == room && peer.id != origin {
                    peer.handle_media_data_out(origin, &data);
                }
            }
            continue;
        }

        // 5. 轮询所有 Peer 直到最近的 timeout（超时点决定 read_timeout 的阻塞时长）
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
                // 还在 ICE 握手中的陌生包，正常现象
                tracing::debug!("no peer accepts UDP input yet");
            }
        }

        // 6. 驱动时间（str0m 内部的重传/keepalive 全靠 Timeout 输入）
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

/// 排空一个 Peer 的输出事件队列（音频 → 入待传播队列；Timeout → 提前结束本轮）。
/// 返回值为该 Peer 最近的定时器，供主循环计算下一轮阻塞时长。
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

/// 从 UDP socket 读一个包并包装成 str0m Input；无包/瞬时错误返回 None（下一轮再读）。
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
