//! STUB FEATURE — delete src/features/example to start your project.
//!
//! sqlx implementation of `domain::Repository`.
//! Uses runtime-checked `sqlx::query_as` (no live DATABASE_URL required at build time).
//!
//! Row → domain mapping is unit-tested directly. End-to-end DB tests are gated
//! behind `#[ignore]` so `cargo test` runs without infra.

use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::features::example::domain::{Error, Item, Repository};

/// sqlx implementation of `domain::Repository`.
#[derive(Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Internal row type — kept private to the repository.
#[derive(Debug, FromRow)]
struct ItemRow {
    id: Uuid,
    name: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<ItemRow> for Item {
    fn from(r: ItemRow) -> Self {
        Item {
            id: r.id,
            name: r.name,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait::async_trait]
impl Repository for PgRepository {
    async fn create(&self, item: &Item) -> Result<(), Error> {
        sqlx::query("INSERT INTO items (id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)")
            .bind(item.id)
            .bind(&item.name)
            .bind(item.created_at)
            .bind(item.updated_at)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(map_sqlx_error)
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Item, Error> {
        let row = sqlx::query_as::<_, ItemRow>(
            "SELECT id, name, created_at, updated_at FROM items WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(Error::NotFound)?;
        Ok(row.into())
    }

    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Item>, Error> {
        let rows = sqlx::query_as::<_, ItemRow>(
            "SELECT id, name, created_at, updated_at \
             FROM items \
             ORDER BY created_at DESC, id DESC \
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(rows.into_iter().map(Item::from).collect())
    }
}

/// Map a `sqlx::Error` to the domain `Error` enum.
///
/// `RowNotFound` becomes the `NotFound` sentinel (mirrors Go's
/// `gorm.ErrRecordNotFound` → `ErrItemNotFound`). Everything else is wrapped
/// as `Internal { cause }` for the boundary to surface as 500.
fn map_sqlx_error(err: sqlx::Error) -> Error {
    match err {
        sqlx::Error::RowNotFound => Error::NotFound,
        other => Error::Internal {
            cause: Some(anyhow::Error::msg(other.to_string())),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row() -> ItemRow {
        ItemRow {
            id: Uuid::nil(),
            name: "alpha".to_string(),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn row_maps_to_domain_item() {
        let item: Item = sample_row().into();
        assert_eq!(item.id, Uuid::nil());
        assert_eq!(item.name, "alpha");
        assert_eq!(item.created_at, OffsetDateTime::UNIX_EPOCH);
    }

    #[test]
    fn row_not_found_translates_to_domain_not_found() {
        let err = map_sqlx_error(sqlx::Error::RowNotFound);
        assert_eq!(err, Error::NotFound);
    }

    #[test]
    fn other_sqlx_errors_become_internal() {
        let err = map_sqlx_error(sqlx::Error::PoolClosed);
        assert!(matches!(err, Error::Internal { cause: Some(_) }));
    }

    // Live-DB tests are gated so `cargo test` runs without infra. Run with:
    //   DATABASE_URL=postgres://... cargo test -- --ignored
    #[ignore]
    #[tokio::test]
    async fn live_db_create_and_get_by_id_roundtrip() {
        use sqlx::postgres::PgPoolOptions;
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("connect");
        let repo = PgRepository::new(pool.clone());
        let id = Uuid::now_v7();
        let item = Item {
            id,
            name: "wave4".to_string(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };
        repo.create(&item).await.expect("insert");
        let got = repo.get_by_id(id).await.expect("fetch");
        assert_eq!(got.name, "wave4");
        sqlx::query("DELETE FROM items WHERE id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .ok();
    }
}
