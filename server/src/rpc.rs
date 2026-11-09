//! JSON-RPC 信令层：与 kraken/mornin.fm 客户端协议兼容。
//!
//! 请求：`POST /rpc` body `{"id": "...", "method": "...", "params": [...]}`
//! 响应：`{"id": "...", "data": ...}` 或 `{"id": "...", "error": {"code": -32000, "description": "..."}}`

use std::sync::mpsc;
use std::time::Instant;

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use str0m::change::{SdpAnswer, SdpOffer};
use str0m::{Candidate, Rtc};

use crate::config::Config;
use crate::engine::{Cmd, NewPeer};
use crate::util::gen_pid;

/// axum State：配置 + 两条通往引擎的通道。
#[derive(Clone)]
pub struct RpcContext {
    pub cfg: Config,
    /// publish 用：承载建好的 Rtc 交给引擎驱动，容量 64 提供背压
    pub new_peer_tx: mpsc::SyncSender<NewPeer>,
    /// 其余所有命令（subscribe/answer/trickle/roster/chat）
    pub cmd_tx: mpsc::Sender<Cmd>,
}

#[derive(serde::Deserialize)]
pub struct RpcRequest {
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Vec<Value>,
}

/// JSON-RPC 入口：分发 + 统一错误包装（异常一律转 `{"error": {...}}`，不让 axum 500）
pub async fn handle_rpc(
    State(ctx): State<RpcContext>,
    Json(req): Json<RpcRequest>,
) -> Json<Value> {
    let id = req.id.clone();
    let result = dispatch(&ctx, &req.method, &req.params).await;
    Json(match result {
        Ok(data) => json!({ "id": id, "data": data }),
        Err(desc) => json!({ "id": id, "error": { "code": -32000, "description": desc } }),
    })
}

type RpcResult = Result<Value, String>;

/// 方法分发。params 为位置参数数组（kraken 协议约定），每个分支注明其形状。
async fn dispatch(ctx: &RpcContext, method: &str, params: &[Value]) -> RpcResult {
    match method {
        "turn" => {
            // params: [userId]（占位，v1 不按用户发临时凭证）
            // RTCIceServer 形状：[{ urls: "stun:..." }]
            Ok(json!(ctx
                .cfg
                .stun_urls
                .iter()
                .map(|u| json!({ "urls": u }))
                .collect::<Vec<_>>()))
        }

        "publish" => {
            // params: [roomId, userId, sdpOffer(JSON 字符串), pid?]
            // pid = 通行证：首次发布由服务器签发；重连（刷新页面）时客户端带旧 pid
            // 回来，服务器按 pid 顶号替换 —— 身份与昵称解耦，同名两人可共存。
            let (room, user, offer_str) = three_strings(params)?;
            let offer: SdpOffer = serde_json::from_str(&offer_str)
                .map_err(|e| format!("bad offer: {e}"))?;

            let pid = match params.get(3).and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                Some(t) => t.to_string(),
                None => gen_pid(),
            };

            let mut rtc = Rtc::builder().build(Instant::now());
            let candidate = Candidate::host(ctx.cfg.public_addr, "udp")
                .map_err(|e| format!("host candidate: {e}"))?;
            rtc.add_local_candidate(candidate);

            let answer = rtc
                .sdp_api()
                .accept_offer(offer)
                .map_err(|e| format!("offer rejected: {e}"))?;

            ctx.new_peer_tx
                .try_send(NewPeer { rtc, room, user, pid: pid.clone() })
                .map_err(|e| format!("engine busy: {e}"))?;

            Ok(json!({ "sdp": answer, "track": params[1].as_str().unwrap_or_default(), "pid": pid }))
        }

        "subscribe" => {
            // params: [pid, ctrl, micOn]
            // 心跳管道：返回 {offer: 有新下行轨时的 offer(否则 null), muted: 该 pid 是否被房主闭麦}；
            // pid 未知 = 会话已被顶/服务器重启 → 直接报错，客户端走重进流程。
            // 兼容：不带第 3 参的是旧版前端 → 按老形状回裸 offer/null（旧 tab 在用户刷新前一直可用）
            let legacy = params.len() < 3;
            let pid = as_string(&params[0]);
            let mic_on = !legacy && matches!(as_string(&params[2]).trim(), "true" | "1");
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::Subscribe { pid, mic_on, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            let tick = spawn_blocking_recv(rx).await??;
            if legacy {
                Ok(json!(tick.offer))
            } else {
                Ok(json!({ "offer": tick.offer, "muted": tick.muted }))
            }
        }

        "answer" => {
            // params: [pid, ctrlTrackId, sdpAnswer(JSON 字符串)]
            let (pid, _ctrl, answer_str) = three_strings(params)?;
            let answer: SdpAnswer =
                serde_json::from_str(&answer_str).map_err(|e| format!("bad answer: {e}"))?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::Answer { pid, answer, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            let ok = spawn_blocking_recv(rx).await?;
            if ok {
                Ok(Value::Null)
            } else {
                Err("no pending subscribe offer for this peer".into())
            }
        }

        "trickle" => {
            // params: [pid, ctrlTrackId, candidate(JSON 字符串)]
            let (pid, _ctrl, cand_str) = three_strings(params)?;
            let candidate = parse_candidate(&cand_str)?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::Trickle { pid, candidate, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            let ok = spawn_blocking_recv(rx).await?;
            if ok {
                Ok(Value::Null)
            } else {
                Err("peer not found".into())
            }
        }

        "roster" => {
            // params: [roomId] → [{ id: "数字 peer id", name: 昵称 }]
            let (room,) = one_string(params)?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::Roster { room, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            Ok(spawn_blocking_recv(rx).await?)
        }

        "chatSend" => {
            // params: [pid, text] —— 署名由服务器按通行证解析，前端传的昵称仅作展示
            let (pid, text) = two_strings(params)?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::ChatSend { pid, text, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            let id = spawn_blocking_recv(rx).await??;
            Ok(json!({ "id": id }))
        }

        "chatFetch" => {
            // params: [roomId, since(上一条消息 id 数字字符串)]
            let (room,) = one_string(params)?;
            let since = params
                .get(1)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| params.get(1).and_then(|v| v.as_u64()))
                .unwrap_or(0);
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::ChatFetch { room, since, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            Ok(spawn_blocking_recv(rx).await?)
        }

        "muteMember" => {
            // params: [房主 pid, 目标成员数字 id, 开关("true"/"1" 闭麦，否则解除)]
            let (pid, target, mute) = three_strings(params)?;
            let target = target
                .trim()
                .parse::<u64>()
                .map_err(|_| "bad target member id".to_string())?;
            let mute = matches!(mute.trim(), "true" | "1");
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::MuteMember { pid, target, mute, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            spawn_blocking_recv(rx).await??;
            Ok(Value::Null)
        }

        other => Err(format!("unknown method: {other}")),
    }
}

/// 浏览器的 RTCIceCandidate JSON：{ candidate: "candidate:... ", sdpMid, sdpMLineIndex }
fn parse_candidate(raw: &str) -> Result<Candidate, String> {
    let sdp_line = if raw.trim_start().starts_with('{') {
        let v: Value = serde_json::from_str(raw).map_err(|e| format!("bad candidate json: {e}"))?;
        v.get("candidate")
            .and_then(|c| c.as_str())
            .ok_or("candidate json missing 'candidate' field")?
            .to_string()
    } else {
        raw.to_string()
    };
    let sdp_line = sdp_line.trim().trim_start_matches("a=").trim().to_string();
    Candidate::from_sdp_string(&sdp_line).map_err(|e| format!("bad candidate: {e}"))
}

/// 位置参数助手：把前 N 个参数取成 String（数字/布尔也转字符串），不足 N 个报协议错误。
fn three_strings(params: &[Value]) -> Result<(String, String, String), String> {
    if params.len() < 3 {
        return Err("expected 3 params".into());
    }
    Ok((
        as_string(&params[0]),
        as_string(&params[1]),
        as_string(&params[2]),
    ))
}

fn two_strings(params: &[Value]) -> Result<(String, String), String> {
    if params.len() < 2 {
        return Err("expected 2 params".into());
    }
    Ok((as_string(&params[0]), as_string(&params[1])))
}

fn one_string(params: &[Value]) -> Result<(String,), String> {
    if params.is_empty() {
        return Err("expected 1 param".into());
    }
    Ok((as_string(&params[0]),))
}

/// 字符串取值兜底：非字符串类型序列化成 JSON 文本（room id "222" 可能被客户端数字传参）
fn as_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// 引擎是 std::thread + std 通道，HTTP 侧用 spawn_blocking 等结果
async fn spawn_blocking_recv<T: Send + 'static>(
    rx: mpsc::Receiver<T>,
) -> Result<T, String> {
    tokio::task::spawn_blocking(move || {
        rx.recv().map_err(|e| format!("engine dropped reply: {e}"))
    })
    .await
    .map_err(|e| format!("join error: {e}"))?
}
