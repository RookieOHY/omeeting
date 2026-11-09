mod config;
mod engine;
mod rpc;
mod static_files;

use std::net::UdpSocket;
use std::sync::mpsc;

use config::Config;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omeeting_server=info,str0m=info,dimpl=info".into()),
        )
        .init();

    let cfg = Config::from_env();
    tracing::info!(?cfg, "o-meeting server starting");

    // HTTP 线程 ↔ 引擎线程 的通道
    let (new_peer_tx, new_peer_rx) = mpsc::sync_channel::<engine::NewPeer>(64);
    let (cmd_tx, cmd_rx) = mpsc::channel::<engine::Cmd>();
    // std Receiver 不可 Clone → 包成 Arc<Mutex<>> 供 supervisor 重生时复用
    let new_peer_rx = std::sync::Arc::new(std::sync::Mutex::new(new_peer_rx));
    let cmd_rx = std::sync::Arc::new(std::sync::Mutex::new(cmd_rx));

    // 引擎线程保底重生：线程若意外退出（panic/卸载等），重建 socket 并拉起新引擎。
    // UDP socket 随线程所有权一起重建——同端口重绑即可（SO_REUSEADDR），客户端流量会重新送达。
    {
        let media_addr = cfg.media_addr;
        let public_addr = cfg.public_addr;
        std::thread::Builder::new()
            .name("sfu-engine-supervisor".into())
            .spawn(move || loop {
                let socket = UdpSocket::bind(media_addr).expect("bind media udp port");
                engine::run(socket, public_addr, new_peer_rx.clone(), cmd_rx.clone());
                tracing::error!("sfu-engine exited unexpectedly, respawning in 1s");
                std::thread::sleep(std::time::Duration::from_secs(1));
            })
            .expect("spawn engine thread");
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    rt.block_on(async move {
        let ctx = rpc::RpcContext { cfg: cfg.clone(), new_peer_tx, cmd_tx };

        let app = axum::Router::new()
            .route("/rpc", axum::routing::post(rpc::handle_rpc))
            .route("/health", axum::routing::get(|| async { "ok" }))
            // 其余路径 → 内嵌前端（SPA 回退 index.html）
            .fallback(static_files::handler)
            // 开发期前端跑在 localhost:5173 等任意源，放行跨源（生产收紧）
            .layer(tower_http::cors::CorsLayer::permissive())
            .with_state(ctx);

        // 生产环境由 Caddy 做 TLS 终止（自动 Let's Encrypt），SFU 本体只跑 HTTP
        let listener = tokio::net::TcpListener::bind(cfg.http_addr)
            .await
            .expect("bind http port");
        tracing::info!(
            http = %cfg.http_addr,
            udp = %cfg.media_addr,
            "HTTP listening — POST /rpc [turn,publish,subscribe,answer,trickle]"
        );
        axum::serve(listener, app).await.expect("axum serve");
    });
}
