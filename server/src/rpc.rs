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

#[derive(Clone)]
pub struct RpcContext {
    pub cfg: Config,
    pub new_peer_tx: mpsc::SyncSender<NewPeer>,
    pub cmd_tx: mpsc::Sender<Cmd>,
}

#[derive(serde::Deserialize)]
pub struct RpcRequest {
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Vec<Value>,
}

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
            // params: [roomId, userId, sdpOffer(JSON 字符串)]
            let (room, user, offer_str) = three_strings(params)?;
            let offer: SdpOffer = serde_json::from_str(&offer_str)
                .map_err(|e| format!("bad offer: {e}"))?;

            let mut rtc = Rtc::builder().build(Instant::now());
            let candidate = Candidate::host(ctx.cfg.public_addr, "udp")
                .map_err(|e| format!("host candidate: {e}"))?;
            rtc.add_local_candidate(candidate);

            let answer = rtc
                .sdp_api()
                .accept_offer(offer)
                .map_err(|e| format!("offer rejected: {e}"))?;

            ctx.new_peer_tx
                .try_send(NewPeer { rtc, room, user })
                .map_err(|e| format!("engine busy: {e}"))?;

            Ok(json!({ "sdp": answer, "track": params[1].as_str().unwrap_or_default() }))
        }

        "subscribe" => {
            // params: [roomId, userId, ctrlTrackId] → 新轨时有 offer，否则 null
            let (room, user, _ctrl) = three_strings(params)?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::Subscribe { room, user, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            let offer = spawn_blocking_recv(rx).await?;
            Ok(json!(offer))
        }

        "answer" => {
            // params: [roomId, userId, ctrlTrackId, sdpAnswer(JSON 字符串)]
            let (room, user, _ctrl, answer_str) = four_strings(params)?;
            let answer: SdpAnswer =
                serde_json::from_str(&answer_str).map_err(|e| format!("bad answer: {e}"))?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::Answer { room, user, answer, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            let ok = spawn_blocking_recv(rx).await?;
            if ok {
                Ok(Value::Null)
            } else {
                Err("no pending subscribe offer for this peer".into())
            }
        }

        "trickle" => {
            // params: [roomId, userId, ctrlTrackId, candidate(JSON 字符串)]
            let (room, user, _ctrl, cand_str) = four_strings(params)?;
            let candidate = parse_candidate(&cand_str)?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::Trickle { room, user, candidate, reply: tx })
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
            // params: [roomId, userId, text]
            let (room, user, text) = three_strings(params)?;
            let (tx, rx) = mpsc::sync_channel(1);
            ctx.cmd_tx
                .send(Cmd::ChatSend { room, user, text, reply: tx })
                .map_err(|e| format!("engine gone: {e}"))?;
            let id = spawn_blocking_recv(rx).await?;
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

fn four_strings(params: &[Value]) -> Result<(String, String, String, String), String> {
    if params.len() < 4 {
        return Err("expected 4 params".into());
    }
    Ok((
        as_string(&params[0]),
        as_string(&params[1]),
        as_string(&params[2]),
        as_string(&params[3]),
    ))
}

fn one_string(params: &[Value]) -> Result<(String,), String> {
    if params.is_empty() {
        return Err("expected 1 param".into());
    }
    Ok((as_string(&params[0]),))
}

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
