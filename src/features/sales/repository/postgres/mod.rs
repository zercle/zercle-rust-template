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
mod tests {
    use super::*;

    fn bank(pairs: &[(i32, i32)]) -> CoinBank {
        pairs.iter().copied().collect()
    }

    #[test]
    fn product_row_maps_to_sale_product() {
        let id = Uuid::nil();
        let got: SaleProduct = SaleProductRow {
            id,
            price_cents: 25,
            stock: 3,
        }
        .into();
        assert_eq!(
            got,
            SaleProduct {
                product_id: id,
                price_cents: 25,
                stock: 3,
            }
        );
    }

    #[test]
    fn coin_bank_round_trips_through_json() {
        let b = bank(&[(5, 2), (25, 3), (100, 1)]);
        let encoded = serde_json::to_string(&b).unwrap();
        assert_eq!(parse_bank(&encoded).unwrap(), b);
    }

    #[test]
    fn empty_coin_bank_parses() {
        assert_eq!(parse_bank("{}").unwrap(), CoinBank::new());
    }

    // --- Live-DB integration tests ---------------------------------------
    //
    // Gated `#[ignore]` so `cargo test` is green without infra. Run with:
    //   cargo test --lib sales -- --include-ignored
    //
    // Setup hard-fails when the database is unreachable and refuses production,
    // applies the merged registry migrations in-suite, and TRUNCATEs the three
    // tables sales touches. A session-scoped advisory lock serializes the suites
    // that share this one database (Go port-spec §6 runs `-p 1` for the same
    // reason). The DSN is `DATABASE_URL` when set, else the `DB_*` config leaves.

    use sqlx::Connection;
    use sqlx::postgres::{PgConnection, PgPoolOptions};

    const TEST_LOCK_KEY: i64 = 0x5a1e_55a1e5;

    struct TestDb {
        pool: PgPool,
        _lock: PgConnection,
    }

    async fn test_db() -> TestDb {
        let url = crate::platform::db::integration_db_url();
        if std::env::var("APP_ENVIRONMENT").as_deref() == Ok("production") || url.contains("/prod")
        {
            panic!("integration tests must not run against production");
        }

        let mut lock = PgConnection::connect(&url)
            .await
            .expect("connect it_sales for advisory lock");
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(TEST_LOCK_KEY)
            .execute(&mut lock)
            .await
            .expect("acquire test advisory lock");

        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(&url)
            .await
            .expect("connect it_sales");
        crate::features::registry::migrator()
            .expect("build merged migrator")
            .run(&pool)
            .await
            .expect("apply registry migrations");
        sqlx::query(
            "TRUNCATE TABLE sales_purchases, machines, catalog_products RESTART IDENTITY CASCADE",
        )
        .execute(&pool)
        .await
        .expect("truncate sales tables");

        TestDb { pool, _lock: lock }
    }

    async fn seed_product(pool: &PgPool, id: Uuid, price_cents: i32, stock: i32) {
        sqlx::query(
            "INSERT INTO catalog_products (id, name, price_cents, stock) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind("widget")
        .bind(price_cents)
        .bind(stock)
        .execute(pool)
        .await
        .expect("seed product");
    }

    async fn seed_machine(pool: &PgPool, id: Uuid, coin_bank: &CoinBank) {
        let json = serde_json::to_string(coin_bank).expect("encode bank");
        sqlx::query("INSERT INTO machines (id, label, coin_bank) VALUES ($1, $2, $3::jsonb)")
            .bind(id)
            .bind("lobby")
            .bind(json)
            .execute(pool)
            .await
            .expect("seed machine");
    }

    async fn stock_of(pool: &PgPool, id: Uuid) -> i32 {
        sqlx::query_scalar::<_, i32>("SELECT stock FROM catalog_products WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("read stock")
    }

    async fn bank_of(pool: &PgPool, id: Uuid) -> CoinBank {
        let raw: String = sqlx::query_scalar("SELECT coin_bank::text FROM machines WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("read bank");
        parse_bank(&raw).expect("parse bank")
    }

    async fn sale_count(pool: &PgPool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sales_purchases")
            .fetch_one(pool)
            .await
            .expect("count sales")
    }

    #[ignore]
    #[tokio::test]
    async fn sales_purchase_decrements_stock_replaces_bank_and_inserts_row() {
        let db = test_db().await;
        let machine = Uuid::now_v7();
        let product = Uuid::now_v7();
        seed_product(&db.pool, product, 25, 5).await;
        seed_machine(&db.pool, machine, &bank(&[(25, 3), (50, 1)])).await;

        let repo = PgRepository::new(db.pool.clone());
        let record = PurchaseRecord {
            id: Uuid::now_v7(),
            machine_id: machine,
            product_id: product,
            price_cents: 25,
            total_inserted_cents: 100,
            change_cents: 75,
            change_coins: vec![50, 25],
            purchased_at: time::OffsetDateTime::now_utc(),
        };
        // Overpay 100 for a 25-cent item: bank {25:3, 50:1} -> change 50+25,
        // remaining {25:2}.
        repo.commit_purchase(machine, product, &record, &bank(&[(25, 2)]))
            .await
            .expect("commit purchase");

        assert_eq!(stock_of(&db.pool, product).await, 4, "stock decremented");
        assert_eq!(
            bank_of(&db.pool, machine).await,
            bank(&[(25, 2)]),
            "bank replaced with composed change remainder"
        );
        assert_eq!(sale_count(&db.pool).await, 1, "sale row inserted");
        let (change_cents, change_coins): (i32, String) = sqlx::query_as(
            "SELECT change_cents, change_coins::text FROM sales_purchases WHERE id = $1",
        )
        .bind(record.id)
        .fetch_one(&db.pool)
        .await
        .expect("read sale");
        assert_eq!(change_cents, 75);
        assert_eq!(
            serde_json::from_str::<Vec<i32>>(&change_coins).unwrap(),
            vec![50, 25]
        );
    }

    #[ignore]
    #[tokio::test]
    async fn sales_commit_unknown_machine_rolls_back_stock_decrement() {
        let db = test_db().await;
        let product = Uuid::now_v7();
        let unknown_machine = Uuid::now_v7(); // never seeded
        seed_product(&db.pool, product, 25, 5).await;

        let repo = PgRepository::new(db.pool.clone());
        let record = PurchaseRecord {
            id: Uuid::now_v7(),
            machine_id: unknown_machine,
            product_id: product,
            price_cents: 25,
            total_inserted_cents: 25,
            change_cents: 0,
            change_coins: Vec::new(),
            purchased_at: time::OffsetDateTime::now_utc(),
        };
        let err = repo
            .commit_purchase(unknown_machine, product, &record, &CoinBank::new())
            .await
            .expect_err("unknown machine must fail");
        assert_eq!(err, Error::MachineNotFound);

        assert_eq!(
            stock_of(&db.pool, product).await,
            5,
            "machine update affected 0 rows -> stock decrement rolled back"
        );
        assert_eq!(sale_count(&db.pool).await, 0, "no sale row committed");
    }

    #[ignore]
    #[tokio::test]
    async fn sales_commit_out_of_stock_affects_zero_rows_and_writes_nothing() {
        let db = test_db().await;
        let machine = Uuid::now_v7();
        let product = Uuid::now_v7();
        seed_product(&db.pool, product, 25, 0).await;
        seed_machine(&db.pool, machine, &bank(&[(25, 1)])).await;

        let repo = PgRepository::new(db.pool.clone());
        let record = PurchaseRecord {
            id: Uuid::now_v7(),
            machine_id: machine,
            product_id: product,
            price_cents: 25,
            total_inserted_cents: 25,
            change_cents: 0,
            change_coins: Vec::new(),
            purchased_at: time::OffsetDateTime::now_utc(),
        };
        let err = repo
            .commit_purchase(machine, product, &record, &bank(&[(25, 1)]))
            .await
            .expect_err("zero stock must fail");
        assert_eq!(err, Error::OutOfStock);

        assert_eq!(stock_of(&db.pool, product).await, 0);
        assert_eq!(bank_of(&db.pool, machine).await, bank(&[(25, 1)]));
        assert_eq!(sale_count(&db.pool).await, 0);
    }

    #[ignore]
    #[tokio::test]
    async fn sales_reads_project_cross_feature_tables_and_surface_not_found() {
        let db = test_db().await;
        let machine = Uuid::now_v7();
        let product = Uuid::now_v7();
        seed_product(&db.pool, product, 50, 7).await;
        seed_machine(&db.pool, machine, &bank(&[(5, 4)])).await;

        let repo = PgRepository::new(db.pool.clone());
        assert_eq!(
            repo.get_product(product).await.unwrap(),
            SaleProduct {
                product_id: product,
                price_cents: 50,
                stock: 7,
            }
        );
        assert_eq!(
            repo.get_machine_bank(machine).await.unwrap(),
            bank(&[(5, 4)])
        );

        assert_eq!(
            repo.get_product(Uuid::now_v7()).await.unwrap_err(),
            Error::ProductNotFound
        );
        assert_eq!(
            repo.get_machine_bank(Uuid::now_v7()).await.unwrap_err(),
            Error::MachineNotFound
        );
    }
}
