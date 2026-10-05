//! Postgres repository adapter for catalog (Go
//! `repository/postgres/repository.go` parity, port-spec §5.1f).
//!
//! Implements `repository::Repository` over a `sqlx::PgPool` against the
//! `catalog_products` table. Uses runtime-checked `sqlx::query_as` (no live
//! `DATABASE_URL` required at build time), matching the crate's adapter style.
//! Row → domain mapping is unit-tested directly; live-DB behavior is covered by
//! the `#[ignore]` integration tests in `repository/postgres/integration.rs`.
//!
//! Migration SQL is embedded from `postgres/migrations/` via
//! `catalog::di::migrations` (Go `//go:embed *.sql` parity).

pub mod cached;

#[cfg(test)]
mod integration;

use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::features::catalog::domain::{Error, Product};
use crate::features::catalog::repository::Repository;

/// `sqlx` implementation of catalog's outbound [`Repository`] port.
#[derive(Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Internal row type — kept private to the repository (Go `models.ProductModel`).
#[derive(Debug, FromRow)]
struct ProductRow {
    id: Uuid,
    name: String,
    price_cents: i32,
    stock: i32,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<ProductRow> for Product {
    fn from(r: ProductRow) -> Self {
        Product {
            id: r.id,
            name: r.name,
            price_cents: r.price_cents,
            stock: r.stock,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait::async_trait]
impl Repository for PgRepository {
    async fn create(&self, product: &Product) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO catalog_products \
             (id, name, price_cents, stock, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(product.id)
        .bind(&product.name)
        .bind(product.price_cents)
        .bind(product.stock)
        .bind(product.created_at)
        .bind(product.updated_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Product, Error> {
        let row = sqlx::query_as::<_, ProductRow>(
            "SELECT id, name, price_cents, stock, created_at, updated_at \
             FROM catalog_products WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(Error::ProductNotFound)?;
        Ok(row.into())
    }

    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Product>, Error> {
        let rows = sqlx::query_as::<_, ProductRow>(
            "SELECT id, name, price_cents, stock, created_at, updated_at \
             FROM catalog_products \
             ORDER BY created_at DESC, id DESC \
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(Product::from).collect())
    }
}

/// Map a `sqlx::Error` to the domain [`Error`] enum.
///
/// A missing row is surfaced by `fetch_optional` as `None`, so the only
/// remaining class is infrastructure failure, wrapped as `Internal { cause }`
/// for the boundary to surface as a 500 (Go wraps non-`ErrRecordNotFound`
/// errors with `%w`).
fn map_sqlx_error(err: sqlx::Error) -> Error {
    Error::Internal {
        cause: Some(anyhow::Error::msg(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row() -> ProductRow {
        ProductRow {
            id: Uuid::nil(),
            name: "alpha".to_string(),
            price_cents: 150,
            stock: 3,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn row_maps_to_domain_product() {
        let product: Product = sample_row().into();
        assert_eq!(product.id, Uuid::nil());
        assert_eq!(product.name, "alpha");
        assert_eq!(product.price_cents, 150);
        assert_eq!(product.stock, 3);
        assert_eq!(product.created_at, OffsetDateTime::UNIX_EPOCH);
    }

    #[test]
    fn infrastructure_errors_become_internal() {
        let err = map_sqlx_error(sqlx::Error::PoolClosed);
        assert!(matches!(err, Error::Internal { cause: Some(_) }));
    }
}
