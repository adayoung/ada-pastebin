use crate::config;
use sqlx::postgres::PgPool;

pub struct AppState {
    pub config: config::AppConfig,
    pub db: PgPool,
}
