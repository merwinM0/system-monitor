use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod auth;
mod collector;
mod static_files;

use auth::{login, AuthState, Claims};
use collector::SystemStats;
use static_files::serve_static;

#[tokio::main]
async fn main() {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    info!("🔧 System Monitor v0.2.0 Starting...");

    // 共享状态
    let auth_state = Arc::new(AuthState::new());

    // 构建路由
    let app = Router::new()
        // 公开路由：登录
        .route("/api/login", post(login))
        // 受保护路由：需要 JWT
        .route("/api/stats", get(get_stats))
        .route("/", get(serve_static))
        .route("/index.html", get(serve_static))
        // CORS 支持（允许前端跨域，开发时用）
        .layer(CorsLayer::permissive())
        // 日志
        .layer(TraceLayer::new_for_http())
        // 共享状态
        .with_state(auth_state);

    // 绑定
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("🚀 Server running on http://{}", addr);
    info!("📱 Login endpoint: POST http://{}/api/login", addr);
    info!("    Body: {{\"username\":\"admin\",\"password\":\"admin123\"}}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// 受保护的 API：自动提取并验证 JWT Claims
async fn get_stats(_claims: Claims) -> axum::Json<SystemStats> {
    let stats = collector::collect_stats().await;
    axum::Json(stats)
}
