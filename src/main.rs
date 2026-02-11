use axum::{
    middleware,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod auth;
mod collector;
mod static_files;

use auth::basic_auth;
use collector::SystemStats;
use static_files::serve_static;

#[tokio::main]
async fn main() {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // 配置账号密码（生产环境建议从环境变量读取）
    let username = std::env::var("MONITOR_USER").unwrap_or_else(|_| "admin".to_string());
    let password = std::env::var("MONITOR_PASS").unwrap_or_else(|_| "123456".to_string());
    
    info!("🔧 System Monitor Starting...");
    info!("👤 Username: {}", username);
    info!("🔒 Password: {}", "*".repeat(password.len()));

    // 构建路由
    let app = Router::new()
        // API 端点
        .route("/api/stats", get(get_stats))
        // 静态文件（前端页面）
        .route("/", get(serve_static))
        .route("/index.html", get(serve_static))
        // 添加认证中间件
        .layer(middleware::from_fn_with_state(
            (username, password),
            basic_auth,
        ))
        // 日志层
        .layer(TraceLayer::new_for_http());

    // 绑定到所有接口（0.0.0.0），允许局域网访问
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("🚀 Server running on http://{}", addr);
    info!("🌐 LAN access: http://<your-ip>:8080");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// API 处理器：返回 JSON 格式的系统数据
async fn get_stats() -> axum::Json<SystemStats> {
    let stats = collector::collect_stats().await;
    axum::Json(stats)
}
