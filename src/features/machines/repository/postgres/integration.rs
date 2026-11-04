//! Live-database integration suite (port-spec §6 test patterns). Runs against a
//! fresh, isolated `it_machines` database on the local compose Postgres.
//!
//! * Production guard: refuses `APP_ENVIRONMENT=production`.
//! * Hard-fails when infra is missing (no skip).
//! * Creates the database via an admin connection, applies the **registry**
//!   migrator in-suite, truncates between cases, and drops the database after.
//!
//! Run with `cargo test --lib machines -- --include-ignored`; the DSN comes
//! from `DATABASE_URL` when set, else from the `DB_*` config leaves
//! (`cp .env.example .env` + `task test-integration` needs no exports).
use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;
use time::OffsetDateTime;
use uuid::Uuid;

use super::PgRepository;
use crate::features::machines::domain::{CoinBank, Error, Machine};
use crate::features::machines::repository::Repository;
use crate::platform::errors::errcodes;

const DB_NAME: &str = "it_machines";

/// Static DDL: sqlx 0.9 requires `&'static str` (or an audited
/// `AssertSqlSafe`) for every query string, and the database name is a
/// compile-time constant.
const DROP_DB_QUERY: &str = "DROP DATABASE IF EXISTS \"it_machines\" WITH (FORCE)";
const CREATE_DB_QUERY: &str = "CREATE DATABASE \"it_machines\"";

/// Shared DSN resolution (`DATABASE_URL` else the `DB_*` config leaves),
/// then pin the isolated test database name so the suite never touches the
/// developer's `app` database.
fn target_url() -> String {
    let mut url = url::Url::parse(&crate::platform::db::integration_db_url())
        .expect("integration DSN must be a valid URL");
    url.set_path(DB_NAME);
    url.to_string()
}

/// Maintenance connection to the `postgres` database, same host/credentials.
fn admin_url(target: &str) -> String {
    let mut url = url::Url::parse(target).expect("DATABASE_URL must be a valid URL");
    url.set_path("/postgres");
    url.to_string()
}

#[tokio::test]
#[ignore = "requires local compose Postgres; creates/drops the it_machines database"]
async fn machines_postgres_integration() {
    // Production guard (port-spec §6): never touch a production database.
    let env = std::env::var("APP_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    assert_ne!(
        env, "production",
        "integration tests must not run against production (APP_ENVIRONMENT=production)"
    );

    let target = target_url();
    let admin = admin_url(&target);

    // Admin pool must be reachable; hard-fail (no skip) when infra is down.
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin)
        .await
        .expect("connect admin Postgres (is `docker compose up postgres` running?)");

    // Fresh isolated database.
    sqlx::query(DROP_DB_QUERY)
        .execute(&admin_pool)
        .await
        .expect("drop stale it_machines");
    sqlx::query(CREATE_DB_QUERY)
        .execute(&admin_pool)
        .await
        .expect("create it_machines");

    // Connect the suite pool and apply the registry's merged migrations
    // in-suite (catalog 1 / machines 2 / sales 3).
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&target)
        .await
        .expect("connect it_machines");
    crate::features::registry::migrator()
        .expect("registry migrator")
        .run(&pool)
        .await
        .expect("apply migrations");

    let repo = Arc::new(PgRepository::new(pool.clone()));
    let truncate = || async {
        sqlx::query("TRUNCATE TABLE machines RESTART IDENTITY CASCADE")
            .execute(&pool)
            .await
            .expect("truncate machines");
    };

    // Case 1: create machine with an initial bank, then read it back.
    truncate().await;
    let id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();
    let mut bank = CoinBank::new();
    bank.insert(25, 2);
    bank.insert(100, 1);
    repo.create(&Machine {
        id,
        label: "lobby".to_string(),
        coin_bank: bank.clone(),
        created_at: now,
        updated_at: now,
    })
    .await
    .expect("create machine");
    let got = repo.get_by_id(id).await.expect("get created machine");
    assert_eq!(got.label, "lobby");
    assert_eq!(
        got.coin_bank, bank,
        "initial bank round-trips through JSONB"
    );

    // Case 2: list pagination.
    truncate().await;
    for label in ["a", "b", "c"] {
        repo.create(&Machine {
            id: Uuid::now_v7(),
            label: label.to_string(),
            coin_bank: CoinBank::new(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        })
        .await
        .expect("seed machine");
    }
    let page1 = repo.list(2, 0).await.expect("list page 1");
    assert_eq!(page1.len(), 2, "limit 2 returns two rows");
    let page2 = repo.list(2, 2).await.expect("list page 2");
    assert_eq!(page2.len(), 1, "offset 2 returns the remaining row");

    // Case 3: unknown id -> NOT_FOUND (sentinel + boundary code).
    let missing = repo.get_by_id(Uuid::now_v7()).await.unwrap_err();
    assert_eq!(missing, Error::MachineNotFound);
    assert_eq!(
        crate::platform::errors::AppError::from(missing).code(),
        errcodes::NOT_FOUND
    );

    // Case 4: restock replaces the bank and writes ONLY coin_bank.
    truncate().await;
    let id = Uuid::now_v7();
    let mut bank = CoinBank::new();
    bank.insert(5, 1);
    let created = OffsetDateTime::now_utc();
    repo.create(&Machine {
        id,
        label: "restock-me".to_string(),
        coin_bank: bank,
        created_at: created,
        updated_at: created,
    })
    .await
    .expect("create machine");
    let before = repo.get_by_id(id).await.expect("read before");
    // Non-coin columns captured before the restock.
    let before_label = before.label.clone();
    let before_created = before.created_at;
    let before_updated = before.updated_at;

    repo.restock_bank(id, &[25, 25, 100])
        .await
        .expect("restock bank");
    let after = repo.get_by_id(id).await.expect("read after");
    assert_eq!(after.coin_bank.get(&5), Some(&1), "existing coins kept");
    assert_eq!(after.coin_bank.get(&25), Some(&2), "restocked 25s added");
    assert_eq!(after.coin_bank.get(&100), Some(&1), "restocked 100 added");
    // Only coin_bank changed: label / created_at / updated_at are untouched.
    assert_eq!(after.label, before_label, "label unchanged");
    assert_eq!(after.created_at, before_created, "created_at unchanged");
    assert_eq!(after.updated_at, before_updated, "updated_at unchanged");

    // Drop the isolated database after the suite.
    pool.close().await;
    sqlx::query(DROP_DB_QUERY)
        .execute(&admin_pool)
        .await
        .expect("drop it_machines");
}
