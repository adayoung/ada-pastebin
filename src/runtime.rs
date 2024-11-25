use crate::config;
use dashmap::DashMap;
use sqlx::postgres::PgPool;

pub struct AppState {
    pub config: config::AppConfig,
    pub db: PgPool,
    pub counter: DashMap<String, u64>,
}
