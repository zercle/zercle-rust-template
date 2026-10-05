//! Postgres read-only aggregation adapter for reporting (Go
//! `repository/postgres/repository.go` parity, port-spec §5.4f).
//!
//! Read-only: reporting owns no schema and never writes. It reads
//! catalog_products, machines, and sales_purchases directly through its own
//! port (the same "deliberate single-database compromise" as sales; a future
//! split replaces this package).
//!
//! Runtime-checked `sqlx::query_as` (no live DATABASE_URL at build time). Row →
//! domain mapping is unit-tested here; the live-DB integration suite lives in
//! the feature's `di` composition root (`cargo test --lib reporting -- --include-ignored`).

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::features::reporting::domain::{Error, MachineSales, Overview};
use crate::features::reporting::port::Repository;

/// Whole-database overview. `SUM(bigint)` yields `numeric`, which sqlx cannot
/// decode into `i64` in binary result format, so the coin-bank sum is cast to
/// `bigint`; every other aggregate is already `bigint`.
const OVERVIEW_QUERY: &str = r#"
SELECT
    (SELECT COUNT(*) FROM catalog_products) AS product_count,
    (SELECT COALESCE(SUM(stock), 0) FROM catalog_products) AS total_stock,
    (SELECT COUNT(*) FROM machines) AS machine_count,
    (SELECT COALESCE(SUM(b.key::bigint * b.value::bigint), 0)::bigint
       FROM machines m, jsonb_each_text(m.coin_bank) AS b(key, value)) AS total_bank_cents,
    (SELECT COUNT(*) FROM sales_purchases) AS purchase_count,
    (SELECT COALESCE(SUM(price_cents), 0) FROM sales_purchases) AS revenue_cents
"#;

/// Leaderboard. INNER JOIN excludes machines with no sales; ties break by
/// revenue desc, purchase count desc, then `m.id` ascending.
const TOP_MACHINES_QUERY: &str = r#"
SELECT m.id AS machine_id, m.label AS label, COUNT(p.id) AS purchase_count, COALESCE(SUM(p.price_cents), 0) AS revenue_cents
FROM machines m
JOIN sales_purchases p ON p.machine_id = m.id
GROUP BY m.id, m.label
ORDER BY revenue_cents DESC, purchase_count DESC, m.id
LIMIT $1
"#;

/// sqlx implementation of the reporting feature's `port::Repository`.
#[derive(Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Internal read-model row for [`OVERVIEW_QUERY`] — kept private.
#[derive(Debug, FromRow)]
struct OverviewRow {
    product_count: i64,
    total_stock: i64,
    machine_count: i64,
    total_bank_cents: i64,
    purchase_count: i64,
    revenue_cents: i64,
}

impl From<OverviewRow> for Overview {
    fn from(r: OverviewRow) -> Self {
        Overview {
            product_count: r.product_count,
            total_stock: r.total_stock,
            machine_count: r.machine_count,
            total_bank_cents: r.total_bank_cents,
            purchase_count: r.purchase_count,
            revenue_cents: r.revenue_cents,
        }
    }
}

/// Internal read-model row for [`TOP_MACHINES_QUERY`] — kept private.
#[derive(Debug, FromRow)]
struct MachineSalesRow {
    machine_id: Uuid,
    label: String,
    purchase_count: i64,
    revenue_cents: i64,
}

impl From<MachineSalesRow> for MachineSales {
    fn from(r: MachineSalesRow) -> Self {
        MachineSales {
            machine_id: r.machine_id,
            label: r.label,
            purchase_count: r.purchase_count,
            revenue_cents: r.revenue_cents,
        }
    }
}

#[async_trait::async_trait]
impl Repository for PgRepository {
    async fn get_overview(&self) -> Result<Overview, Error> {
        let row = sqlx::query_as::<_, OverviewRow>(OVERVIEW_QUERY)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(row.into())
    }

    async fn get_top_machines(&self, limit: i32) -> Result<Vec<MachineSales>, Error> {
        let rows = sqlx::query_as::<_, MachineSalesRow>(TOP_MACHINES_QUERY)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(MachineSales::from).collect())
    }
}

/// Map a `sqlx::Error` to the domain `Error` enum. Everything becomes
/// `Internal { cause }` for the boundary to surface as 500.
fn map_sqlx_error(err: sqlx::Error) -> Error {
    Error::Internal {
        cause: Some(anyhow::Error::msg(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_overview_row() -> OverviewRow {
        OverviewRow {
            product_count: 2,
            total_stock: 9,
            machine_count: 3,
            total_bank_cents: 265,
            purchase_count: 4,
            revenue_cents: 350,
        }
    }

    fn sample_machine_row() -> MachineSalesRow {
        MachineSalesRow {
            machine_id: Uuid::from_u128(0xabc),
            label: "lobby".to_string(),
            purchase_count: 2,
            revenue_cents: 300,
        }
    }

    #[test]
    fn overview_row_maps_to_domain() {
        let overview: Overview = sample_overview_row().into();
        assert_eq!(overview.product_count, 2);
        assert_eq!(overview.total_stock, 9);
        assert_eq!(overview.machine_count, 3);
        assert_eq!(overview.total_bank_cents, 265);
        assert_eq!(overview.purchase_count, 4);
        assert_eq!(overview.revenue_cents, 350);
    }

    #[test]
    fn machine_row_maps_to_domain() {
        let machine: MachineSales = sample_machine_row().into();
        assert_eq!(machine.machine_id, Uuid::from_u128(0xabc));
        assert_eq!(machine.label, "lobby");
        assert_eq!(machine.purchase_count, 2);
        assert_eq!(machine.revenue_cents, 300);
    }

    #[test]
    fn sqlx_errors_become_internal() {
        let err = map_sqlx_error(sqlx::Error::PoolClosed);
        assert!(matches!(err, Error::Internal { cause: Some(_) }));
    }
}
