//! 静态托管：前端 dist 编译进二进制（rust-embed）。
//! GET /room1 这类 SPA 路由回退到 index.html；/rpc、/health 由显式路由优先。
//!
//! 缓存策略（关键）：index.html 必须 no-store，否则浏览器会拿旧版页面
//! 引旧版 JS（Vite 的 hash 文件名永远变了它也不去重新拉）——表现为「改了没生效」。
//! 带内容 hash 的 assets 才允许长缓存。

use axum::http::Uri;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../web/dist"]
struct WebAssets;

fn serve(path: &str) -> Response {
    match WebAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            // hash 指纹文件（xxx-abc123.js）→ 可永久缓存；否则（index.html/裸路径）→ 不缓存
            let cache = if path.contains('-') {
                "public, max-age=31536000, immutable"
            } else {
                "no-store, must-revalidate"
            };
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                [(header::CACHE_CONTROL, cache)],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

pub async fn handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    // 带扩展名的文件找不到就 404；否则 SPA 回退 index.html（history 路由）
    if path.contains('.') {
        serve(path)
    } else {
        serve("index.html")
    }
}
