//! The single feature registry (Go `internal/features/features.go` parity,
//! port-spec §1): it lists every feature and exposes the two enumeration points
//! the composition root and the migration runner need.
//!
//! * [`list`] — every feature in registration order.
//! * [`register_all`] — wire the features and merge their HTTP + gRPC
//!   contributions into one [`Wired`].
//! * [`migrator`] — the merged, version-sorted migration set (Go
//!   `MigrationSources` + `fsmerge`, §1).
//!
//! Gating lives in each feature's `register` (Go §5i: a disabled feature
//! returns early and registers nothing — no routes, no providers).

use std::{borrow::Cow, collections::HashSet, sync::Arc};

use axum::Router;
use redis::aio::ConnectionManager;
use sqlx::{
    PgPool,
    migrate::{Migration, Migrator},
};

use crate::features::{
    catalog::di as catalog_di, machines::di as machines_di, reporting::di as reporting_di,
    sales::di as sales_di,
};
use crate::platform::{
    config::Config,
    server::{GrpcRouter, Wired},
};

/// The `register` function a feature exposes to the registry. Features receive
/// the shared composition inputs (`&Arc<Config>`, `PgPool`, the valkey
/// connection handle) plus the accumulating gRPC router, and return their HTTP
/// routes + the gRPC router with their services added.
///
/// tonic has no `Router::merge`, so the gRPC router is threaded through every
/// feature (each chains `add_service`), mirroring Go's single shared Echo
/// instance; HTTP routers merge directly (each is pre-nested under `/api/v1`).
pub type RegisterFn = fn(&Arc<Config>, PgPool, ConnectionManager, GrpcRouter) -> Wired;

/// One feature's registry entry (Go `features.Feature` parity).
pub struct Feature {
    /// Stable feature name (used for diagnostics).
    pub name: &'static str,
    /// Composition entry point (see [`RegisterFn`]).
    pub register: RegisterFn,
    /// Embedded migrations this feature owns, in the single global version
    /// namespace; `None` when the feature owns no schema (Go `Migrations fs.FS`
    /// nil parity).
    pub migrations: Option<fn() -> Vec<Migration>>,
}

/// The registered features, in registration order (Go `features.List`).
///
/// Registered order is the migration version order (Go §1): catalog owns
/// version 1, machines 2, sales 3; reporting owns no schema (it reads the
/// other features' tables) so it contributes no migrations.
pub fn list() -> Vec<Feature> {
    vec![
        Feature {
            name: "catalog",
            register: catalog_di::register_with_grpc,
            migrations: Some(catalog_di::migrations),
        },
        Feature {
            name: "machines",
            register: machines_di::register_with_grpc,
            migrations: Some(machines_di::migrations),
        },
        Feature {
            name: "sales",
            register: sales_di::register_with_grpc,
            migrations: Some(sales_di::migrations),
        },
        Feature {
            name: "reporting",
            register: reporting_di::register_with_grpc,
            migrations: None,
        },
    ]
}

/// Wire every registered feature into one [`Wired`], in list order (Go
/// `RegisterAll`). Each feature self-gates on its `enabled` config flag.
pub fn register_all(cfg: &Arc<Config>, db: PgPool, valkey: ConnectionManager) -> Wired {
    let mut http = Router::new();
    let mut grpc = crate::platform::server::empty_grpc_router();
    for feature in list() {
        let wired = (feature.register)(cfg, db.clone(), valkey.clone(), grpc);
        http = http.merge(wired.http);
        grpc = wired.grpc;
    }
    Wired { http, grpc }
}

/// Build the merged migration set across every registered feature that owns
/// schema, sorted by version (Go `MigrationSources` + `fsmerge`, §1).
///
/// A reversible version contributes both its up and down entries, so the
/// duplicate check is keyed by `(version, direction)`: two features may not
/// claim the same version in the same direction (versions are one namespace).
/// sqlx 0.8 exposes no public constructor from a `Vec<Migration>`, but
/// `Migrator`'s fields are `pub` (so the `migrate!` macro can build it); we
/// start from [`Migrator::DEFAULT`] to keep its defaults (`locking = true`,
/// `ignore_missing = false`).
pub fn migrator() -> anyhow::Result<Migrator> {
    let mut all: Vec<Migration> = Vec::new();
    for feature in list() {
        if let Some(source) = feature.migrations {
            all.extend(source());
        }
    }
    all.sort_by_key(|m| m.version);
    let mut seen: HashSet<(i64, bool)> = HashSet::new();
    for migration in &all {
        let key = (
            migration.version,
            migration.migration_type.is_up_migration(),
        );
        if !seen.insert(key) {
            anyhow::bail!(
                "duplicate migration version {} across registered features",
                migration.version
            );
        }
    }
    Ok(Migrator {
        migrations: Cow::Owned(all),
        ..Migrator::DEFAULT
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_registers_the_four_features_in_version_order() {
        let features = list();
        let names: Vec<&str> = features.iter().map(|f| f.name).collect();
        assert_eq!(names, vec!["catalog", "machines", "sales", "reporting"]);
        assert!(features[0].migrations.is_some(), "catalog owns schema");
        assert!(features[1].migrations.is_some(), "machines owns schema");
        assert!(features[2].migrations.is_some(), "sales owns schema");
        assert!(features[3].migrations.is_none(), "reporting owns no schema");
    }

    #[test]
    fn migrator_merges_and_keeps_sqlx_defaults() {
        let m = migrator().expect("registry builds");
        // Versions 1/2/3 are catalog/machines/sales.
        assert!(m.version_exists(1));
        assert!(m.version_exists(2));
        assert!(m.version_exists(3));
        // Defaults preserved from `Migrator::DEFAULT`.
        assert!(m.locking);
        assert!(!m.ignore_missing);
        // The up direction has no duplicate versions across features.
        let mut up_versions: Vec<i64> = m
            .iter()
            .filter(|mig| mig.migration_type.is_up_migration())
            .map(|mig| mig.version)
            .collect();
        up_versions.sort_unstable();
        let before = up_versions.len();
        up_versions.dedup();
        assert_eq!(before, up_versions.len(), "up versions must be unique");
    }
}
