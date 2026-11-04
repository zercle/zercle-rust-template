//! sqlx implementation of machines `repository::Repository` (postgres repository; Go
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
use crate::features::machines::repository::Repository;

/// sqlx implementation of machines `repository::Repository`.
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

/// Static queries: sqlx 0.9 requires a `&'static str` (or an audited
/// `AssertSqlSafe`) for every query string.
const SELECT_BY_ID_QUERY: &str = "SELECT id, label, coin_bank::text AS coin_bank, created_at, updated_at \
     FROM machines WHERE id = $1";

const LIST_QUERY: &str = "SELECT id, label, coin_bank::text AS coin_bank, created_at, updated_at \
     FROM machines ORDER BY created_at DESC, id DESC LIMIT $1 OFFSET $2";

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
        let row = sqlx::query_as::<_, MachineRow>(SELECT_BY_ID_QUERY)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?
            .ok_or(Error::MachineNotFound)?;
        row_to_machine(row)
    }

    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Machine>, Error> {
        let rows = sqlx::query_as::<_, MachineRow>(LIST_QUERY)
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
mod tests;

#[cfg(test)]
mod integration;
