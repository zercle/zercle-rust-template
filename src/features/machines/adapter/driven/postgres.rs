//! sqlx implementation of machines `port::Repository` (driven adapter; Go
//! `repository/postgres/repository.go` parity, port-spec §5.2f).
//!
//! Table `machines`; `coin_bank` is a JSONB object keyed by denomination in
//! cents (`{"25":2,"100":1}`). Runtime-checked queries, so no live
//! `DATABASE_URL` at build time. `coin_bank` is selected as `::text` and parsed
//! through `serde_json` (`BTreeMap<i32,i32>` stringifies its integer keys
//! exactly like the JSONB object the Go adapter writes).
//!
//! `restock_bank` locks the row (`SELECT ... FOR UPDATE`) and updates **only**
//! `coin_bank`; the use case re-reads afterwards for the authoritative bank
//! (port-spec §5.2d/§5.2f). End-to-end DB tests are `#[ignore]`d behind a
//! fresh isolated database (see the test module).

use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::features::machines::domain::{CoinBank, Error, Machine, add_coins};
use crate::features::machines::port::Repository;

/// sqlx implementation of machines `port::Repository`.
#[derive(Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Internal row type; `coin_bank` arrives as JSON text (see module docs).
#[derive(Debug, FromRow)]
struct MachineRow {
    id: Uuid,
    label: String,
    coin_bank: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

const SELECT_COLUMNS: &str = "id, label, coin_bank::text AS coin_bank, created_at, updated_at";

fn row_to_machine(row: MachineRow) -> Result<Machine, Error> {
    let coin_bank: CoinBank = serde_json::from_str(&row.coin_bank).map_err(map_serde_error)?;
    Ok(Machine {
        id: row.id,
        label: row.label,
        coin_bank,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

#[async_trait::async_trait]
impl Repository for PgRepository {
    async fn create(&self, machine: &Machine) -> Result<(), Error> {
        let bank = serde_json::to_string(&machine.coin_bank).map_err(map_serde_error)?;
        sqlx::query(
            "INSERT INTO machines (id, label, coin_bank, created_at, updated_at) \
             VALUES ($1, $2, $3::jsonb, $4, $5)",
        )
        .bind(machine.id)
        .bind(&machine.label)
        .bind(bank)
        .bind(machine.created_at)
        .bind(machine.updated_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Machine, Error> {
        let sql = format!("SELECT {SELECT_COLUMNS} FROM machines WHERE id = $1");
        let row = sqlx::query_as::<_, MachineRow>(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(Error::MachineNotFound)?;
        row_to_machine(row)
    }

    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Machine>, Error> {
        let sql = format!(
            "SELECT {SELECT_COLUMNS} FROM machines \
             ORDER BY created_at DESC, id DESC LIMIT $1 OFFSET $2"
        );
        let rows = sqlx::query_as::<_, MachineRow>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(row_to_machine).collect()
    }

    async fn restock_bank(&self, id: Uuid, coins: &[i32]) -> Result<(), Error> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        // Row lock so concurrent restocks cannot lose an update.
        let raw: Option<String> =
            sqlx::query_scalar("SELECT coin_bank::text FROM machines WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        let Some(raw) = raw else {
            return Err(Error::MachineNotFound);
        };
        let current: CoinBank = serde_json::from_str(&raw).map_err(map_serde_error)?;
        let bank_after = add_coins(&current, coins);
        let bank = serde_json::to_string(&bank_after).map_err(map_serde_error)?;
        // Go `Select("coin_bank")`: only coin_bank is written (updated_at is not).
        sqlx::query("UPDATE machines SET coin_bank = $2::jsonb WHERE id = $1")
            .bind(id)
            .bind(bank)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        tx.commit().await.map_err(map_sqlx_error)
    }
}

/// Map a `sqlx::Error` to the domain `Error` enum: `RowNotFound` becomes the
/// `MachineNotFound` sentinel; everything else is wrapped `Internal` for 500.
fn map_sqlx_error(err: sqlx::Error) -> Error {
    match err {
        sqlx::Error::RowNotFound => Error::MachineNotFound,
        other => Error::Internal {
            cause: Some(anyhow::Error::msg(other.to_string())),
        },
    }
}

fn map_serde_error(err: serde_json::Error) -> Error {
    Error::Internal {
        cause: Some(anyhow::Error::msg(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(bank_json: &str) -> MachineRow {
        MachineRow {
            id: Uuid::nil(),
            label: "lobby".to_string(),
            coin_bank: bank_json.to_string(),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn jsonb_text_maps_to_domain_bank() {
        let machine = row_to_machine(row(r#"{"25":2,"100":1}"#)).expect("parse");
        assert_eq!(machine.id, Uuid::nil());
        assert_eq!(machine.label, "lobby");
        assert_eq!(machine.coin_bank.get(&25), Some(&2));
        assert_eq!(machine.coin_bank.get(&100), Some(&1));
    }

    #[test]
    fn empty_object_maps_to_empty_bank() {
        let machine = row_to_machine(row("{}")).expect("parse");
        assert!(machine.coin_bank.is_empty());
    }

    #[test]
    fn malformed_jsonb_is_internal() {
        let err = row_to_machine(row("not json")).unwrap_err();
        assert!(matches!(err, Error::Internal { cause: Some(_) }));
    }

    #[test]
    fn row_not_found_translates_to_machine_not_found() {
        assert_eq!(
            map_sqlx_error(sqlx::Error::RowNotFound),
            Error::MachineNotFound
        );
    }

    #[test]
    fn other_sqlx_errors_become_internal() {
        assert!(matches!(
            map_sqlx_error(sqlx::Error::PoolClosed),
            Error::Internal { cause: Some(_) }
        ));
    }
}

/// Live-database integration suite (port-spec §6 test patterns). Runs against a
/// fresh, isolated `it_machines` database on the local compose Postgres.
///
/// * Production guard: refuses `APP_ENVIRONMENT=production`.
/// * Hard-fails when infra is missing (no skip).
/// * Creates the database via an admin connection, applies the **registry**
///   migrator in-suite, truncates between cases, and drops the database after.
///
/// Run with:
/// `DATABASE_URL=postgres://postgres:postgres@localhost:5432/it_machines \
///  cargo test --lib machines -- --include-ignored`
#[cfg(test)]
mod integration {
    use std::sync::Arc;

    use sqlx::postgres::PgPoolOptions;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::PgRepository;
    use crate::features::machines::domain::{CoinBank, Error, Machine};
    use crate::features::machines::port::Repository;
    use crate::platform::errors::errcodes;

    const DB_NAME: &str = "it_machines";
    const DEFAULT_URL: &str = "postgres://postgres:postgres@localhost:5432/it_machines";

    fn target_url() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_URL.to_string())
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
        sqlx::query(&format!(
            "DROP DATABASE IF EXISTS \"{DB_NAME}\" WITH (FORCE)"
        ))
        .execute(&admin_pool)
        .await
        .expect("drop stale it_machines");
        sqlx::query(&format!("CREATE DATABASE \"{DB_NAME}\""))
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
        sqlx::query(&format!(
            "DROP DATABASE IF EXISTS \"{DB_NAME}\" WITH (FORCE)"
        ))
        .execute(&admin_pool)
        .await
        .expect("drop it_machines");
    }
}
