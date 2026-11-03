//! Postgres repository adapter for sales (Go
//! `repository/postgres/repository.go` parity, port-spec §5.3f): cross-feature
//! reads of catalog/machines through this feature's own row projections, and
//! the purchase transaction.
//!
//! Deliberate single-database compromise: sales reaches `catalog_products` and
//! `machines` directly through its own repository interface rather than calling other features'
//! use cases; a distributed deployment would swap this port for cross-service
//! calls/saga. `sales_purchases` carries no foreign keys to either table.
//!
//! May reference only this feature's domain and port
//! (`tests/architecture.rs`: repository-impl-ignores-usecase-and-handler).
//!
//! Encoding note: the crate's `sqlx` build has no `json` feature, so the JSONB
//! columns are carried as `::text` / `::jsonb`-cast strings parsed with
//! `serde_json` — the coin bank is a `{denomination: count}` object.

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::features::sales::domain::{CoinBank, Error, PurchaseRecord, SaleProduct};
use crate::features::sales::repository::Repository;

/// sqlx implementation of the sales feature's `repository::Repository`.
#[derive(Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Sales' own read projection over `catalog_products` (Go `ProductRefModel`).
#[derive(Debug, FromRow)]
struct SaleProductRow {
    id: Uuid,
    price_cents: i32,
    stock: i32,
}

impl From<SaleProductRow> for SaleProduct {
    fn from(r: SaleProductRow) -> Self {
        SaleProduct {
            product_id: r.id,
            price_cents: r.price_cents,
            stock: r.stock,
        }
    }
}

/// Sales' own read projection over `machines`, carrying the JSONB column as
/// text so it can be parsed without the `sqlx` `json` feature.
#[derive(Debug, FromRow)]
struct MachineBankRow {
    coin_bank: String,
}

#[async_trait::async_trait]
impl Repository for PgRepository {
    async fn get_product(&self, product_id: Uuid) -> Result<SaleProduct, Error> {
        let row = sqlx::query_as::<_, SaleProductRow>(
            "SELECT id, price_cents, stock FROM catalog_products WHERE id = $1",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(Error::ProductNotFound)?;
        Ok(row.into())
    }

    async fn get_machine_bank(&self, machine_id: Uuid) -> Result<CoinBank, Error> {
        let row = sqlx::query_as::<_, MachineBankRow>(
            "SELECT coin_bank::text AS coin_bank FROM machines WHERE id = $1",
        )
        .bind(machine_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(Error::MachineNotFound)?;
        parse_bank(&row.coin_bank)
    }

    /// Commit the purchase, decrement stock, and replace the machine bank in one
    /// transaction (Go §5.3f). Any error — including the machine update
    /// affecting zero rows — rolls the whole transaction back.
    async fn commit_purchase(
        &self,
        machine_id: Uuid,
        product_id: Uuid,
        record: &PurchaseRecord,
        bank_after: &CoinBank,
    ) -> Result<(), Error> {
        let bank_json = serde_json::to_string(bank_after).map_err(internal)?;
        let change_json = serde_json::to_string(&record.change_coins).map_err(internal)?;

        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        // 1. Lock the product row so a concurrent purchase cannot oversell.
        let locked = sqlx::query_as::<_, SaleProductRow>(
            "SELECT id, price_cents, stock FROM catalog_products WHERE id = $1 FOR UPDATE",
        )
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if locked.is_none() {
            return Err(Error::ProductNotFound);
        }

        // 2. Decrement stock; a race that lost the last unit affects 0 rows.
        let decremented = sqlx::query(
            "UPDATE catalog_products SET stock = stock - 1 WHERE id = $1 AND stock > 0",
        )
        .bind(product_id)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if decremented.rows_affected() == 0 {
            return Err(Error::OutOfStock);
        }

        // 3. Replace the machine coin bank with the composed change bank; a
        //    missing machine affects 0 rows and rolls back the decrement above.
        let machine = sqlx::query(
            "UPDATE machines SET coin_bank = $2::jsonb, updated_at = now() WHERE id = $1",
        )
        .bind(machine_id)
        .bind(&bank_json)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if machine.rows_affected() == 0 {
            return Err(Error::MachineNotFound);
        }

        // 4. Insert the sale.
        sqlx::query(
            "INSERT INTO sales_purchases \
             (id, machine_id, product_id, price_cents, total_inserted_cents, change_cents, change_coins, purchased_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8)",
        )
        .bind(record.id)
        .bind(machine_id)
        .bind(product_id)
        .bind(record.price_cents)
        .bind(record.total_inserted_cents)
        .bind(record.change_cents)
        .bind(&change_json)
        .bind(record.purchased_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        tx.commit().await.map_err(map_sqlx_error)
    }
}

/// Parse a JSONB coin bank (`{denomination: count}`) read as text.
fn parse_bank(raw: &str) -> Result<CoinBank, Error> {
    serde_json::from_str(raw).map_err(internal)
}

/// Every `sqlx` failure other than the mapped sentinels is an internal error.
fn map_sqlx_error(err: sqlx::Error) -> Error {
    internal(err)
}

fn internal(err: impl std::fmt::Display) -> Error {
    Error::Internal {
        cause: Some(anyhow::Error::msg(err.to_string())),
    }
}

#[cfg(test)]
mod tests;
