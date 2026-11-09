//! 跨模块小工具：时间戳、随机身份。无业务语义，谁都可以用。

/// 当前 Unix 时间，毫秒。系统时钟倒退等异常时返回 0（调用方按“未知时间”兜底）。
pub fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 会话通行证 ID：随机 64bit 的 hex（16 字符）。
///
/// 进房时签发一次、客户端存 sessionStorage；之后所有请求按它认人。
/// 可猜测性要求不高（房间本身就是“拿链接才能进”的门槛），64bit 足够防撞车。
pub fn gen_pid() -> String {
    let mut b = [0u8; 8];
    if getrandom::fill(&mut b).is_err() {
        // 兜底：时间纳秒（理论上不会走到，getrandom 只在系统熵源缺失时失败）
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        b = n.to_le_bytes();
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}
