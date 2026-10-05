//! PostgreSQL connection pool + readiness checker.
//!
//! Mirrors `internal/infrastructure/db/{db,health}.go` (structure.md §10).
//!
//! * [`new_pool`] builds a tuned [`sqlx::PgPool`] from [`Config`], pings the database, and
//!   returns the live pool. On ping failure the pool is closed before returning the error.
//! * [`PgChecker`] implements [`crate::platform::health::Checker`] and pings the pool for readiness.

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

/// Local hostnames a live-infra suite may target. Shared by the guard below.
#[cfg(test)]
const INTEGRATION_LOCAL_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

/// Resolve the Postgres DSN for an in-crate live-infra integration suite.
///
/// Mirrors the Go template's integration harnesses, which call `config.Load()`
/// rather than reading `DATABASE_URL`: an explicit `DATABASE_URL` (a local
/// throwaway database) wins; otherwise the DSN is derived from the same
/// `Config::load()` DB leaves the `server`/`migrate` binaries use
/// ([`Config::db_conn_string`]). That makes `cp .env.example .env && task
/// test-integration` work with no hand-exported environment (Go Taskfile parity).
///
/// Test-only. Hard-fails (never skips) on `APP_ENVIRONMENT=production` or a
/// non-local database host, so the production/locality guard is enforced for
/// every suite that resolves a DSN through here.
#[cfg(test)]
pub fn integration_db_url() -> String {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        Config::load()
            .expect("hard-fail: load config to derive the integration database DSN")
            .db_conn_string()
    });
    assert_local_integration_db(&url);
    url
}

/// Production / remote guard for integration DSNs.
#[cfg(test)]
fn assert_local_integration_db(url: &str) {
    if std::env::var("APP_ENVIRONMENT").as_deref() == Ok("production") {
        panic!("integration tests must not run against production (APP_ENVIRONMENT=production)");
    }
    let host = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string));
    match host.as_deref() {
        Some(h) if INTEGRATION_LOCAL_HOSTS.contains(&h) => {}
        other => panic!(
            "refusing non-local integration database host {other:?}; integration tests must \
             target a local throwaway database"
        ),
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
