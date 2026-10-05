# zercle-rust-template

Opinionated Rust service template: clean (DDD) architecture per feature, **axum** (HTTP) +
**tonic** (gRPC) + **sqlx** (PostgreSQL) + **redis** (Valkey), OpenTelemetry tracing, Prometheus
metrics, `tracing` logging, and a Valkey cache-aside example — with four layered features
(catalog, machines, sales, reporting) forming a small distributed-vending-machines demo to copy
or delete. It is the Rust sibling of `zercle-go-template`, mirroring its feature registry, layer
layout, error envelope, and executable dependency gates.

## Prerequisites

- **Rust** stable (pinned via `rust-toolchain.toml`; edition 2024, `rust-version` 1.85)
- **protoc** (for the proto compile in `build.rs`; tonic-build has no bundled compiler)
- **Docker/Podman**
- [Task](https://taskfile.dev/installation/) (the command runner; raw `cargo` also works)
- PostgreSQL 18+ (via container)
- Valkey 9+ (via container)

## Quick start

```bash
cp .env.example .env
docker compose up -d postgres valkey
task migrate-up        # or: cargo run --bin migrate -- up
task run               # or: cargo run --bin server
```

The server listens on `0.0.0.0:8080` (HTTP) and `0.0.0.0:50051` (gRPC). Health probes:
`/healthz`, `/readyz`, `/metrics`.

For the full containerised stack (migrations as a one-shot `migrate` service, then `server` plus
an optional observability profile):

```bash
docker compose up -d                            # postgres + valkey + migrate + server
docker compose --profile observability up -d    # + otel-collector, prometheus, grafana
```

## Directory tree

```
zercle-rust-template/
├── .github/
│   ├── dependabot.yml          # weekly cargo + actions + docker updates
│   └── workflows/              # ci.yml (fmt/clippy/architecture/unit/integration/build), cd.yml, security.yml
├── deployments/
│   ├── kustomize/
│   │   ├── base/               # deployment + service + configmap + secret
│   │   └── overlays/development/
│   └── observability/          # otel-collector-config.yaml, prometheus.yml
├── proto/                      # one gRPC contract per feature (generated in build.rs)
│   ├── catalog/v1/catalog.proto
│   ├── machines/v1/machines.proto
│   ├── sales/v1/sales.proto
│   └── reporting/v1/reporting.proto
├── src/
│   ├── main.rs                 # bin `server`: load config → lib::run
│   ├── lib.rs                  # crate root + module declarations
│   ├── bin/migrate.rs          # bin `migrate`: up / down [N] / force / version
│   ├── app.rs                  # composition root: platform + features::registry::register_all
│   ├── api/                    # published facade namespace (outward-only)
│   │   ├── mod.rs
│   │   ├── v1.rs               # re-exports every feature's contract types
│   │   └── errcodes.rs         # published error-code constants
│   ├── platform/               # cross-cutting shell — never imports features
│   │   ├── config.rs           # Config + Load + validate (explicit leaf-binding table)
│   │   ├── db.rs               # PgPool + ping + readiness checker
│   │   ├── valkey.rs           # redis ConnectionManager + ping + readiness checker
│   │   ├── errcodes.rs         # canonical error-code constants
│   │   ├── errors.rs           # AppError + IntoResponse + grpc status mapper
│   │   ├── health.rs           # Checker trait + Registry
│   │   ├── telemetry.rs        # tracing + OTel + Prometheus init
│   │   ├── middleware/         # request-id, access-log, recover, cors
│   │   └── server/             # AppState + run() + http/grpc builders + shutdown
│   └── features/
│       ├── registry.rs         # feature registry: the single enumeration point
│       ├── catalog/            # global product pool (price + stock) + Valkey cache-aside
│       ├── machines/           # vending machines + their coin banks
│       ├── sales/              # purchases: price a sale, compose change, commit
│       └── reporting/          # read-only cross-feature GET /reports/summary (no migrations)
│           # catalog, machines, sales, and reporting each repeat the layer layout below:
│           ├── contract/       # inbound wire types (LEAF; published via crate::api::v1)
│           ├── domain/         # entities + sentinel errors (innermost)
│           ├── repository/     # outbound repository interface (Repository trait)
│           │   └── postgres/   # sqlx repository (catalog adds the cache-aside decorator,
│           │       └── migrations/   # feature-owned SQL, one global version namespace
│           ├── usecase/        # inbound Service trait + Usecase implementation
│           ├── handler/        # axum handlers + tonic service
│           └── di.rs           # wiring + sentinel→AppError mapping
├── tests/
│   ├── common/mod.rs           # shared helpers for integration + e2e tests
│   ├── architecture.rs         # executable dependency gates (layering rules)
│   └── e2e.rs                  # e2e: boots the full app (self-skips w/o infra)
├── build.rs                    # tonic-build: compile every proto/*/v1/*.proto
├── Cargo.toml
├── Cargo.lock                  # committed for reproducible builds
├── rust-toolchain.toml
├── rustfmt.toml
├── Taskfile.yml                # cargo wrapper (build, test, migrate, docker-build, cover)
├── Containerfile               # multi-stage musl + distroless (server image)
├── Containerfile.migrate       # multi-stage migrate image
├── compose.yml                 # postgres + valkey + migrate + server + observability profile
├── config.yaml
├── .env.example
└── LICENSE
```

## Architecture overview

The template follows **clean (DDD) architecture** inside each feature, with all dependencies
pointing inward:

```
consumer services ──> api::v1 ──> features/*/contract    (published contract, outward-only)
handler ──> usecase::Service ──> repository::Repository <── repository/postgres
all layers ──> domain (entities + sentinel errors)
platform/* ── cross-cutting, never imports features/**
```

- `domain` holds entities and sentinel errors (innermost; no crate-internal dependencies).
- `contract` holds the canonical inbound wire types (`serde` + `validator` only) — the single
  source of the API shapes.
- `repository` declares the outbound `Repository` trait; it references only its own feature's
  domain.
- `usecase` declares the inbound `Service` trait (speaking contract types) and its `Usecase`
  implementation, which owns the domain ↔ contract mapping and the single validation path
  (e.g. wire-id parsing) shared by HTTP and gRPC.
- `handler` (axum handlers, tonic service) translates transport ↔ contract and calls
  `usecase::Service`; `repository/postgres` satisfies `repository::Repository` with sqlx and
  owns the persistence models and SQL migrations.
- `di` is the composition edge: it wires repository → use case → handlers, nests HTTP routes
  under `/api/v1`, registers the feature's gRPC service on the platform's tonic router, and maps
  the domain sentinel errors to the shared `AppError`.

**Published inbound contract.** `crate::api::v1` re-exports every feature's `contract` types plus
the error codes in `crate::api::errcodes`, so *other services* can construct payloads and
interpret the `{"error": code, "message": msg}` envelope without importing server internals.
Internal code never imports `crate::api`.

**Executable dependency gates.** `tests/architecture.rs` scans every `crate::`-rooted `use`
statement across `src/` and fails when a layer reaches sideways or outward: facade imports,
domain/contract purity, repository and usecase allowlists, handler/repository separation, registry
purity, and platform's feature-agnosticism. It runs as its own CI job and in `task test-architecture`.

**Feature registry.** `src/features/registry.rs` is the **single enumeration point**: `list()`
holds one `Feature` — `name`, `register`, `migrations` — per feature, `register_all` folds every
feature into one `Wired` (merged axum router + accumulating tonic router), and `migrator()` feeds
the migration runner. Adding or deleting a feature is therefore one entry in that list plus the
feature's own directory. That order is also the migration order: `catalog` owns schema version 1,
`machines` version 2, `sales` version 3; `reporting` registers with no migrations because it owns
no schema of its own.

**Migrations are feature-owned.** Each feature's SQL lives in its
`repository/postgres/migrations/` and is embedded per feature; the `migrate` binary builds one
merged, version-sorted `sqlx::Migrator` from every registered feature's migrations via
`registry::migrator()` (Go `MigrationSources` + `fsmerge` parity), so deleting a feature deletes
its schema with it. Migration versions are a **single namespace across all features**, not per
feature: the next migration added to any feature takes the next free version, and a duplicate
`(version, direction)` across features fails the build of the migrator. `task migrate-up` /
`migrate-down` run the self-contained `cargo run --bin migrate …` runner; `migrate-create
FEATURE=<name> NAME=…` hand-writes an empty reversible pair under the named feature's migrations
directory using the next free global version (the Taskfile notes `sqlx migrate add -r` as the
per-directory, timestamp-versioned alternative when sqlx-cli is installed).

Configuration is loaded from `config.yaml` and the environment (no prefix) into a typed,
validated struct via the `config` crate + `validator`. `CATALOG_ENABLED`, `MACHINES_ENABLED`,
`SALES_ENABLED`, and `REPORTING_ENABLED` gate each feature: when false its providers, routes, and
sentinels are not registered at all (a disabled feature registers nothing). Name/label and
page-size limits (`CATALOG_MAX_NAME_LENGTH`, `CATALOG_MAX_PAGE_SIZE`, `MACHINES_MAX_LABEL_LENGTH`,
`MACHINES_MAX_PAGE_SIZE`) are enforced in the use-case layer, so a deployment can raise them
without touching request validation. The reporting top-machines bounds
(`REPORTING_DEFAULT_TOP_MACHINES`, `REPORTING_MAX_TOP_MACHINES`) are enforced the same way.

### Routes

| Method | Path | Feature | Purpose |
|---|---|---|---|
| POST | `/api/v1/products` | catalog | add a product to the global pool |
| GET | `/api/v1/products` | catalog | list products (paginated) |
| GET | `/api/v1/products/{id}` | catalog | fetch one product |
| POST | `/api/v1/machines` | machines | register a machine with an initial coin bank |
| GET | `/api/v1/machines` | machines | list machines (paginated) |
| GET | `/api/v1/machines/{id}` | machines | fetch one machine |
| POST | `/api/v1/machines/{id}/bank` | machines | restock a machine's coin bank |
| POST | `/api/v1/purchases` | sales | buy a product: price, compose change, commit |
| GET | `/api/v1/reports/summary` | reporting | cross-feature totals + top machines by revenue |

Health and observability endpoints (`/healthz`, `/readyz`, `/metrics`) are served by
`platform/server`.

Each feature also exposes the same operations over **gRPC** (the Rust template ships gRPC for
every feature; Go has no gRPC layer): `catalog.v1.CatalogService` (CreateProduct, GetProduct,
ListProducts), `machines.v1.MachinesService` (CreateMachine, GetMachine, ListMachines,
RestockBank), `sales.v1.SalesService` (Purchase), and `reporting.v1.ReportingService` (Summary) —
see `proto/*/v1/*.proto`. The platform's tonic layer applies the same request-id + access-log +
panic-recovery interceptor that the HTTP middleware stack applies (`platform/server/grpc_interceptor.rs`),
mirroring HTTP middleware parity.

Every HTTP failure — handler errors and framework errors alike — is served in the
`{"error": code, "message": msg}` envelope, with codes from `platform::errcodes` (published as
`api::errcodes`). The gRPC side maps the same `AppError` to a `tonic::Code` and redacts internal
causes from the wire message.

**Cross-feature boundaries.** Features never import each other; each owns its domain, contract,
and repository interface. `sales` consumes catalog and machines data only through its own
`repository::Repository`, whose postgres implementation reads the `catalog_products` and `machines`
tables directly and commits the sale in one transaction. This is a deliberate single-database
compromise — the tables are shared, but the interface is the seam: a future service split replaces
that one implementation without touching the sales domain or usecase layer. Stock is a
**global pool** (decrementing a product affects every machine), while per-machine product slots
are the documented extension if the demo grows. `reporting` demonstrates the read-only side of
the same seam: it aggregates totals across all three features' tables through its own repository
interface and owns no schema of its own.

## Caching (Valkey cache-aside)

`catalog`'s `CachedRepository` decorates the sqlx repository with a cache-aside read of
`GetByID` only, under the key `catalog:product:<uuid>`, so the use-case layer stays cache-unaware.
A miss loads through the inner repository and, on **success only**, writes the entry back with
the TTL `VALKEY_TTL` (bound to `valkey.ttl`, default `30s`). A not-found result is never cached;
`List` is never cached (a page depends on the whole table's write history); `Create` does not
invalidate (a brand-new id cannot already be cached). Read errors degrade to the inner
repository and writes are best-effort — a cache fault never fails a request.

Rust divergence from Go: the Go template composes a `valkeyaside` **client-side-caching** client
(`aside.Get(ctx, ttl, key, loader)`), whose invalidation is driven by Valkey's tracking protocol.
This cache-aside decorator uses plain cache-aside `GET` / `SETEX` over the shared `ConnectionManager` — no
client-side tracking — so entries expire by TTL and are not server-invalidated. The cached wire
shape mirrors Go's `cachedProduct`.

## Adding and deleting features

To add a feature:

1. Copy or author `src/features/<name>/` (the layers described above).
2. Add one entry to `registry::list()` in `src/features/registry.rs` — `name`, `register`, and
   `migrations` when it owns schema. Migrations are numbered in one global namespace, so take the
   next free version across all features.
3. Register its config in `src/platform/config.rs` (struct + leaf binding) and add its re-exports
   in `src/api/v1.rs` before publishing.

To replace the demo features (catalog, machines, sales, reporting):

1. Remove the feature directories under `src/features/` you are replacing.
2. Remove their entries from `registry::list()` in `src/features/registry.rs`.
3. Replace their re-exports in `src/api/v1.rs` with your feature's contract types.
4. Delete the `catalog:` / `machines:` / `sales:` / `reporting:` blocks from `config.yaml` and the
   matching `CATALOG_*` / `MACHINES_*` / `SALES_*` / `REPORTING_*` lines from `.env.example`.
5. Drop the matching `CATALOG_ENABLED` / … leaves from `leaf_bindings()` and the `Config` struct
   in `src/platform/config.rs`.

## Testing

```bash
# Unit tests (hermetic, mocked; no infra needed)
task test              # or: cargo test --all-targets

# Clean-architecture dependency gates (layering rules; no infra needed)
task test-architecture # or: cargo test --test architecture

# Integration tests: the #[ignore]-gated live-infra suites (db, valkey, repository roundtrip)
docker compose up -d postgres valkey
task test-integration  # or: cargo test --all-targets -- --include-ignored --test-threads=1

# End-to-end test (boots the full app; needs infra + migrations applied)
task migrate-up
task test-e2e          # or: cargo test --test e2e -- --include-ignored
```

`cargo test` runs the unit suites and the architecture gates; the integration suites are gated
behind `#[ignore]` (the Rust equivalent of Go's build tags) and only run with `--include-ignored`.
Unlike the self-skipping e2e test, the integration suites **hard-fail** when their infrastructure
is missing and **refuse to run against production** (`APP_ENVIRONMENT=production`) or a non-local
`DATABASE_URL`; they read `DATABASE_URL` directly and apply the registry's merged migrations
in-suite. The e2e test (`tests/e2e.rs`) boots the full stack, drives one flow per feature
(catalog → machines → sales → reporting) plus the `/healthz` and `/readyz` probes and one
error-envelope assertion, and skips cleanly when Postgres + Valkey are unreachable.

Other quality gates:

```bash
task lint        # or: cargo clippy --all-targets -- -D warnings
task fmt         # or: cargo fmt
task fmt-check   # or: cargo fmt --all -- --check
task cover       # or: cargo llvm-cov --workspace --all-targets  (requires cargo-llvm-cov; 60% line gate)
```

## CI (GitHub Actions)

`.github/workflows/ci.yml` runs on every push/PR to `main`/`develop`:

1. **fmt** — `cargo fmt --all -- --check`.
2. **clippy** — `cargo clippy --all-targets --locked -- -D warnings`.
3. **architecture** — the layering gates from `tests/architecture.rs` (fast, dedicated signal).
4. **unit** — full test run with `cargo-llvm-cov` coverage; lcov + HTML artifacts uploaded,
   gated at 60% line coverage.
5. **integration** — full suite (`--include-ignored`) against real `postgres:18-alpine` +
   `valkey:9-alpine` service containers; migrations applied via the `migrate` binary.
6. **build** — release build of both binaries + a `docker build` of the Containerfile.

All actions are pinned to commit SHAs and checkouts run with `persist-credentials: false`;
Dependabot keeps the pins, crates, and base images updated weekly. `security.yml` runs a weekly
Trivy filesystem scan plus `cargo-audit` (Rust's counterpart to Go's `govulncheck`); `cd.yml`
publishes multi-arch server **and** migrate images to ghcr.io on a `v*` tag.

## Deployment

- **Container build**:
  - Server: `task docker-build` (`docker build -f Containerfile -t zercle-rust-template:latest .`)
  - Migrate: `task docker-build-migrate` (`docker build -f Containerfile.migrate …`)
  Both are multi-stage (`messense/rust-musl-cross` builder → `distroless/static` non-root final).
- **Local containers**: `docker compose up -d` (add `--profile observability` for OTel +
  Prometheus + Grafana). The compose stack runs the `migrate` service once before the `server`
  service starts.
- **Kubernetes**: `kubectl apply -k deployments/kustomize/overlays/development`. The base
  `Deployment` runs the distroless image as non-root with `readOnlyRootFilesystem: true`; secrets
  hold `DB_PASSWORD` / `VALKEY_PASSWORD`.
- Kubernetes manifests (`deployments/kustomize`) and the observability profile
  (`deployments/observability`, `compose.yml`) are Rust-template extras — the Go template omits
  both.

## Migration from `zercle-go-template`

- **Env vars.** The server binaries bind the same env names as the Go template (`APP_NAME`,
  `DB_HOST`, `VALKEY_PASSWORD`, `OTEL_TRACES_SAMPLER_ARG`, `CATALOG_ENABLED`, …) over
  `config.yaml`. One nuance: server binaries (and the `migrate` binary) bind the `DB_*` leaves
  (`DB_NAME`, `DB_HOST`, …) — there is no `DATABASE_URL` leaf in the config table; the
  **integration test harnesses** read `DATABASE_URL` directly instead. The CI integration job
  therefore sets both `DB_*` (for the `migrate` binary) and `DATABASE_URL` (for the harness).
- **Config keys.** All config keys are identical — `config.yaml` round-trips.
- **gRPC.** There is no Go gRPC layer to match; this template's `proto/*/v1/*.proto` are its own
  published RPC contracts (server-only, `build_client(false)`).
- **Health endpoints.** `/healthz`, `/readyz`, and the metrics path `/metrics` match.

## License

MIT — see `LICENSE`.
