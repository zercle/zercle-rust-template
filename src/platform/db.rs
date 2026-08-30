//! PostgreSQL connection pool + readiness checker.
//!
//! Mirrors `internal/infrastructure/db/{db,health}.go` (structure.md §10).
//!
//! * [`new_pool`] builds a tuned [`sqlx::PgPool`] from [`Config`], pings the database, and
//!   returns the live pool. On ping failure the pool is closed before returning the error.
//! * [`PgChecker`] implements [`shared::health::Checker`] and pings the pool for readiness.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;

use crate::{platform::config::Config, platform::health::Checker};

/// Build a tuned [`sqlx::PgPool`] from `cfg` and ping it before returning.
///
/// Mirrors Go's pool tuning + ping-on-startup contract.
pub async fn new_pool(cfg: &Config) -> Result<sqlx::PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(cfg.db.max_conns)
        .min_connections(cfg.db.min_conns)
        .max_lifetime(cfg.db_max_conn_life())
        .idle_timeout(cfg.db_max_conn_idle())
        .acquire_timeout(cfg.db_connect_timeout())
        .connect(&cfg.db_conn_string())
        .await
        .context("connect postgres")?;

    // Belt-and-braces ping; sqlx already pings on `connect_with`, but we mirror Go's explicit
    // PingContext so a transient startup failure surfaces a clean error.
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .context("ping postgres")?;

    Ok(pool)
}

/// Readiness checker that pings the underlying [`sqlx::PgPool`].
#[derive(Clone)]
pub struct PgChecker {
    pool: sqlx::PgPool,
}

impl PgChecker {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Checker for PgChecker {
    fn name(&self) -> &'static str {
        "postgres"
    }

    async fn check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .context("ping postgres")
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn checker_name_is_postgres() {
        // We never connect: a `PgChecker` is constructed in production code with a real pool;
        // for the name-only assertion, hand-construct an unconnected pool by going through the
        // builder's `connect_lazy` so we never touch the network.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://postgres:postgres@localhost:5432/app")
            .expect("lazy pool must construct without network");
        let c = PgChecker::new(pool);
        assert_eq!(c.name(), "postgres");
    }

    #[tokio::test]
    #[ignore = "requires a live Postgres at localhost:5432 with user=postgres password=postgres db=app"]
    async fn check_pings_real_database() {
        let cfg = Config::load().expect("load config");
        let pool = new_pool(&cfg).await.expect("connect");
        let c = PgChecker::new(pool);
        c.check().await.expect("live DB ping");
    }
}
