use axum::{
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
use axum::http::Uri;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    info!("System Monitor Starting...");

    let auth_state = Arc::new(AuthState::new());

    let app = Router::new()
        .route("/api/login", post(login))
        .route("/api/stats", get(get_stats))
        // 静态文件服务 - 捕获所有路径
        .route("/*path", get(serve_static))
        .route("/", get(serve_static))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(auth_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_stats(_claims: Claims) -> axum::Json<SystemStats> {
    let stats = collector::collect_stats().await;
    axum::Json(stats)
}
