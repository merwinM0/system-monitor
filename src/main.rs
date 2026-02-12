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
mod tui;

use auth::{AuthState, Claims, login};
use collector::SystemStats;
use static_files::serve_static;

#[tokio::main]
async fn main() {
    // 初始化日志（降低级别，避免干扰 TUI）
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::WARN)
        .with_writer(std::io::stderr) // 日志输出到 stderr，不影响 TUI
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // 初始化终端
    let mut terminal = match tui::init_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to initialize terminal: {}", e);
            return;
        }
    };

    // 确保退出时恢复终端
    let _guard = scopeguard::guard((), |_| {
        let _ = tui::restore_terminal();
    });

    // 配置
    let port = 8080;
    let username = std::env::var("MONITOR_USER").unwrap_or_else(|_| "user".to_string());
    let password = std::env::var("MONITOR_PASS").unwrap_or_else(|_| "user123".to_string());

    // 获取网络接口信息
    let interfaces = network::get_network_interfaces();

    // 过滤只显示局域网接口
    let lan_interfaces: Vec<_> = interfaces
        .into_iter()
        .filter(|i| network::is_lan_ip(&i.ip))
        .collect();

    // 绘制初始界面
    let username_for_ui = username.clone();
    let password_for_ui = password.clone();
    let interfaces_for_ui = lan_interfaces.clone();

    terminal
        .draw(|f| {
            tui::draw_ui(
                f,
                port,
                &username_for_ui,
                &password_for_ui,
                &interfaces_for_ui,
            );
        })
        .unwrap();

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

    // 绑定到所有接口
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // 更新界面：显示启动成功
    terminal
        .draw(|f| {
            tui::draw_ui(
                f,
                port,
                &username_for_ui,
                &password_for_ui,
                &interfaces_for_ui,
            );
        })
        .unwrap();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // 优雅关闭
    let server = axum::serve(listener, app);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        // 错误时恢复终端再输出
        let _ = tui::restore_terminal();
        eprintln!("服务错误: {}", e);
        return;
    }

    // 绘制关闭界面
    terminal.draw(|f| tui::draw_shutdown(f)).unwrap();

    // 短暂延迟让用户看到关闭提示
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
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
