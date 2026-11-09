//! 房间账本：房主登记 + 聊天/弹幕环形缓冲。
//!
//! 房间没有实体 ——「有人在」才有房间。账本只记两件事：
//! * `owners`：第一个进房的通行证 = 房主（绑 pid 不绑昵称，同名不混淆）；
//! * `chats`：每房间最多 200 条的滚动聊天记录。
//!
//! 引擎检测到房间清空时调用 [`Rooms::clear`]，下次进房即新房间的第一轮。

use std::collections::{HashMap, VecDeque};

use serde_json::{json, Value};

use crate::util::unix_millis;

/// 每房聊天/弹幕缓冲：环形 200 条，id 全局从 1 起自增
#[derive(Debug, Default)]
struct ChatBuf {
    next_id: u64,
    msgs: VecDeque<Value>,
}

/// 单条消息在缓冲里的字段上限（超长直接拒绝，防止刷屏/恶意占内存）
const MAX_MSG_LEN: usize = 500;
/// 每房最多保留的聊天条数
const CHAT_KEEP: usize = 200;

#[derive(Debug, Default)]
pub struct Rooms {
    /// 房间名 → 房主通行证
    owners: HashMap<String, String>,
    /// 房间名 → 聊天缓冲
    chats: HashMap<String, ChatBuf>,
}

impl Rooms {
    /// 第一个进房的会话登记为房主；已有登记时不动（房主离开再看， room 清空才重置）。
    pub fn claim_owner(&mut self, room: &str, pid: &str) {
        self.owners.entry(room.to_string()).or_insert_with(|| pid.to_string());
    }

    /// 该通行证在此房间是否为房主
    pub fn is_owner(&self, room: &str, pid: &str) -> bool {
        self.owners.get(room).map(|o| o == pid).unwrap_or(false)
    }

    /// 房间清空：房主登记 + 聊天记录一并作废，下次进房算新一轮
    pub fn clear(&mut self, room: &str) {
        self.owners.remove(room);
        self.chats.remove(room);
    }

    /// 消息入账。`pid` 只是记录进消息体（前端按它判定“是不是我发的”），署名用 `user`。
    /// 返回分配的消息 id；文本为空或超长返回 `Err`。
    pub fn push_chat(&mut self, room: &str, user: &str, pid: &str, text: &str) -> Result<u64, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("empty message".into());
        }
        if text.chars().count() > MAX_MSG_LEN {
            return Err("message too long".into());
        }
        let buf = self.chats.entry(room.to_string()).or_default();
        let id = buf.next_id;
        buf.next_id += 1;
        buf.msgs.push_back(json!({
            "id": id,
            "user": user,
            "text": text,
            "ts": unix_millis(),
            "pid": pid,
        }));
        // 环形缓冲：满了扔最旧的（不减 next_id，保证 id 单调）
        while buf.msgs.len() > CHAT_KEEP {
            buf.msgs.pop_front();
        }
        Ok(id)
    }

    /// 增量拉取：返回消息 id > `since` 的全部消息（首次 since=0 即全量历史）
    pub fn fetch_chat(&self, room: &str, since: u64) -> Vec<Value> {
        self.chats
            .get(room)
            .map(|b| {
                b.msgs
                    .iter()
                    .filter(|m| m["id"].as_u64().unwrap_or(0) > since)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}
