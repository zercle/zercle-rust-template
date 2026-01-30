use sqlx::postgres::PgPoolOptions;
use sqlx::Pool;
use sqlx::Postgres;
use crate::internal::infrastructure::config::Config;

pub type DbPool = Pool<Postgres>;

pub struct Database;

impl Database {
    pub async fn connect(config: &Config) -> anyhow::Result<DbPool> {
        let pool = PgPoolOptions::new()
            .max_connections(config.database.max_open_conns)
            .min_connections(config.database.max_idle_conns)
            .connect(&config.connection_string())
            .await?;

        Ok(pool)
    }
}
