use axum::{
    Router,
    routing::{get, post},
};
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

mod auth;
mod collector;
mod network;
mod static_files;
mod ui;

use auth::{AuthState, Claims, login};
use axum::http::Uri;
use collector::SystemStats;
use static_files::serve_static;

#[tokio::main]
async fn main() {
    // 初始化日志（仅记录到文件，不输出到终端避免干扰 UI）
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::WARN) // 降低日志级别，减少终端干扰
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // 清屏并打印 banner
    print!("\x1b[2J\x1b[1;1H"); // ANSI 清屏 + 光标置顶
    ui::print_banner();

    // 配置
    let port = 8080;
    let username = std::env::var("MONITOR_USER").unwrap_or_else(|_| "user".to_string());
    let password = std::env::var("MONITOR_PASS").unwrap_or_else(|_| "user123".to_string());

    // 获取网络接口信息（包含类型）
    let interfaces = network::get_network_interfaces();

    // 调试：打印所有接口（开发时启用）
    // network::print_network_debug();

    // 过滤只显示局域网接口
    let lan_interfaces: Vec<_> = interfaces
        .into_iter()
        .filter(|i| network::is_lan_ip(&i.ip))
        .collect();

    // 打印服务器信息
    ui::print_server_info(port, &lan_interfaces);

    // 打印认证信息
    ui::print_auth_info(&username, &password);

    // 打印访问提示
    ui::print_access_tips();

    // 构建服务
    let auth_state = Arc::new(AuthState::new_with_credentials(username, password));

    let app = Router::new()
        .route("/api/login", post(login))
        .route("/api/stats", get(get_stats))
        .route("/*path", get(serve_static))
        .route("/", get(serve_static))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(auth_state);

    // 绑定到所有接口（关键：允许局域网访问）
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // 打印启动信息
    println!(
        "{}[✓]{} 服务启动成功，按 Ctrl+C 停止服务",
        ui::GREEN,
        ui::RESET
    );
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // 优雅关闭
    let server = axum::serve(listener, app);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        eprintln!("服务错误: {}", e);
    }

    ui::print_shutdown();
}

async fn get_stats(_claims: Claims) -> axum::Json<SystemStats> {
    let stats = collector::collect_stats().await;
    axum::Json(stats)
}

/// 处理关闭信号
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
