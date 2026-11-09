use std::net::{IpAddr, SocketAddr};

/// 服务配置：全部可由环境变量覆盖，默认本机跑通。
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP（JSON-RPC 信令）
    pub http_addr: SocketAddr,
    /// 共享 UDP 媒体端口（所有房间的 ICE 都复用这一个端口）
    pub media_addr: SocketAddr,
    /// 对外宣告的候选地址（生产部署设 PUBLIC_IP）
    pub public_addr: SocketAddr,
    /// 下发给浏览器的 STUN 列表
    pub stun_urls: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let http_port: u16 = env_parse("HTTP_PORT", 7070);
        let media_port: u16 = env_parse("MEDIA_PORT", 7071);
        let ip: IpAddr = std::env::var("PUBLIC_IP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(IpAddr::from([127, 0, 0, 1]));

        Self {
            http_addr: SocketAddr::from(([0, 0, 0, 0], http_port)),
            media_addr: SocketAddr::from(([0, 0, 0, 0], media_port)),
            public_addr: SocketAddr::new(ip, media_port),
            stun_urls: vec![
            "stun:stun.l.google.com:19302".to_string(),
            "stun:stun.qq.com:3478".to_string(),
            "stun:stun.miwifi.com:3478".to_string(),
        ],
        }
    }
}

fn env_parse<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}
