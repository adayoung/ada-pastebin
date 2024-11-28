use crate::config;
use dashmap::{DashMap, DashSet};
use sqlx::postgres::PgPool;
use tokio::signal;
use tower_cookies::Key;

pub struct AppState {
    pub cloudflare_q: DashSet<String>,
    pub config: config::AppConfig,
    pub cookie_key: Key,
    pub counter: DashMap<String, u64>,
    pub db: PgPool,
}

// https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
pub async fn shutdown_signal() {
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
