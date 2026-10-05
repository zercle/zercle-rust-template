//! Cache-aside decorator over the catalog [`Repository`] port (Go
//! `repository/postgres/cached_repository.go` parity, port-spec §5.1j).
//!
//! Caches `GetByID` only, under the key `catalog:product:<uuid>`, using the
//! platform [`ConnectionManager`] and `serde_json`. A cache miss loads through
//! the inner repository and, on **success only**, writes the entry back with a
//! TTL. A not-found result is never cached (the loader error leaves the entry
//! empty); `List` is never cached (a page depends on the whole table's write
//! history, so a stale page would misreport global stock); `Create` does not
//! invalidate (a brand-new id cannot already be cached).
//!
//! Rust divergence from Go: the Go template composes a `valkeyaside`
//! client-side-caching client (`aside.Get(ctx, ttl, key, loader)`), whose
//! invalidation is driven by Valkey's tracking protocol. This port uses plain
//! cache-aside `GET` / `SETEX` over the shared `ConnectionManager` — no
//! client-side tracking — so read errors degrade to the inner repository and
//! writes are best-effort (a cache fault never fails a request). The wire shape
//! of the cached payload mirrors Go's `cachedProduct`.

use std::{sync::Arc, time::Duration};

use redis::{AsyncCommands, aio::ConnectionManager};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::features::catalog::domain::{Error, Product};
use crate::features::catalog::port::Repository;

/// Cache key prefix (Go `productCachePrefix`, verbatim, §5.1j).
const PRODUCT_CACHE_PREFIX: &str = "catalog:product:";

/// Fallback product-cache TTL: the port-spec §5.1j / Go `setDefaults` default
/// of `30s`. Production TTL is
/// [`Config::valkey_ttl`](crate::platform::config::Config::valkey_ttl)
/// (`valkey.ttl`, env `VALKEY_TTL`), wired at the composition edge in
/// `catalog::di`; this constant is retained as the documented default for the
/// cache-TTL assertion.
pub const DEFAULT_PRODUCT_CACHE_TTL: Duration = Duration::from_secs(30);

/// JSON wire shape of a cached product (Go `cachedProduct`, verbatim §5.1j).
/// Deliberately independent of both domain and contract: it is a cache format,
/// not an API promise.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedProduct {
    id: String,
    name: String,
    price_cents: i32,
    stock: i32,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

impl CachedProduct {
    fn from_domain(product: &Product) -> Self {
        Self {
            id: product.id.to_string(),
            name: product.name.clone(),
            price_cents: product.price_cents,
            stock: product.stock,
            created_at: product.created_at,
            updated_at: product.updated_at,
        }
    }

    fn to_domain(&self) -> Result<Product, Error> {
        Ok(Product {
            // Re-parse the id (Go `toDomain` does the same); a corrupt cache
            // entry is treated as an internal fault by the caller.
            id: Uuid::parse_str(&self.id).map_err(|_| Error::InvalidId)?,
            name: self.name.clone(),
            price_cents: self.price_cents,
            stock: self.stock,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Cache-aside wrapper around an inner [`Repository`].
#[derive(Clone)]
pub struct CachedRepository {
    inner: Arc<dyn Repository>,
    conn: ConnectionManager,
    ttl: Duration,
}

impl CachedRepository {
    pub fn new(inner: Arc<dyn Repository>, conn: ConnectionManager, ttl: Duration) -> Self {
        Self { inner, conn, ttl }
    }

    /// Best-effort write of `product` to the cache with the configured TTL.
    /// Serialization / transport failures are logged and swallowed: the read
    /// path must never fail because the cache is unavailable.
    async fn store(&self, key: &str, product: &Product) {
        let payload = match serde_json::to_string(&CachedProduct::from_domain(product)) {
            Ok(payload) => payload,
            Err(err) => {
                tracing::warn!(error = %err, "catalog cache serialization failed");
                return;
            }
        };
        // A zero TTL would make `SETEX` invalid; the TTL config is validated
        // `>= 1s` in Go, so clamp defensively to one second.
        let seconds = self.ttl.as_secs().max(1);
        let mut conn = self.conn.clone();
        if let Err(err) = conn.set_ex::<_, _, ()>(key, payload, seconds).await {
            tracing::warn!(error = %err, "catalog cache SETEX failed");
        }
    }
}

#[async_trait::async_trait]
impl Repository for CachedRepository {
    async fn create(&self, product: &Product) -> Result<(), Error> {
        // No invalidation: a newly created id cannot already be cached.
        self.inner.create(product).await
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Product, Error> {
        let key = format!("{PRODUCT_CACHE_PREFIX}{id}");
        let mut conn = self.conn.clone();

        match conn.get::<_, Option<String>>(&key).await {
            Ok(Some(raw)) => match serde_json::from_str::<CachedProduct>(&raw) {
                Ok(cached) => match cached.to_domain() {
                    Ok(product) => return Ok(product),
                    Err(err) => {
                        tracing::warn!(error = %err, key = %key, "catalog cache entry corrupt; reloading");
                    }
                },
                Err(err) => {
                    tracing::warn!(error = %err, key = %key, "catalog cache entry undecodable; reloading");
                }
            },
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(error = %err, "catalog cache GET failed; reading through");
            }
        }

        // Miss (or degraded cache): load through the inner repository. On error
        // (not found, infrastructure) nothing is written, so a not-found result
        // is never cached.
        let product = self.inner.get_by_id(id).await?;
        self.store(&key, &product).await;
        Ok(product)
    }

    async fn list(&self, limit: i32, offset: i32) -> Result<Vec<Product>, Error> {
        // Never cached: a page depends on the whole table's write history.
        self.inner.list(limit, offset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: Uuid) -> Product {
        let mut p = Product {
            id,
            name: "cola".to_string(),
            price_cents: 150,
            stock: 7,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        p.created_at = OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("ts");
        p.updated_at = p.created_at;
        p
    }

    #[test]
    fn cached_payload_round_trips_through_json() {
        let product = sample(Uuid::nil());
        let json = serde_json::to_string(&CachedProduct::from_domain(&product)).unwrap();
        assert!(
            json.contains("\"created_at\":\"2023-11-14T22:13:20Z\""),
            "{json}"
        );
        let back = serde_json::from_str::<CachedProduct>(&json).unwrap();
        assert_eq!(back.to_domain().unwrap(), product);
    }

    #[test]
    fn corrupt_id_is_rejected_on_decode() {
        let cached = CachedProduct {
            id: "not-a-uuid".to_string(),
            name: "x".to_string(),
            price_cents: 1,
            stock: 1,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        assert_eq!(cached.to_domain().unwrap_err(), Error::InvalidId);
    }
}
