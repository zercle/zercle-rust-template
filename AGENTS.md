# Repository Guidelines

Repo-specific guidance for AI coding agents working in `zercle-rust-template`.

`README.md` holds the architecture rationale and the feature-replacement checklist. `Taskfile.yml`,
`rustfmt.toml`, and `.github/workflows/ci.yml` are the executable source of truth — verify
anything here against them.

## Project Overview

An opinionated Rust **HTTP + gRPC service template**: clean (DDD) architecture per feature,
axum HTTP, tonic gRPC, sqlx over PostgreSQL, a Valkey cache-aside example, `tracing` +
OpenTelemetry tracing + Prometheus metrics. Four layered features form a **distributed-vending-machines
demo** — `catalog` (global product pool), `machines` (vending machines + coin banks), `sales`
(purchases), and `reporting` (read-only cross-feature summary) — meant to be copied or replaced;
see README §"Adding and deleting features".

Consumers (other services) import only `crate::api::v1` to construct payloads and interpret the
`{"error": code, "message": msg}` envelope; internal code never imports `crate::api`.

## Architecture & Data Flow

Clean (DDD) architecture **per feature**, all dependencies pointing inward. This direction is
**enforced by an executable test**, not convention alone (`tests/architecture.rs`, runs in
`task test-architecture` and as its own CI job).

```
consumer services ──> api::v1 ──> features/*/contract    (published contract, outward-only)
adapter/driving ──> application::Service ──> port::Repository <── adapter/driven/postgres
all layers ──> domain (entities + sentinel errors; innermost)
platform/* ── feature-agnostic, never imports features/**
```

A request flows: `src/main.rs` loads config → `src/app.rs::build` wires the platform
(`telemetry → postgres → valkey → health → AppState`) → `features::registry::register_all` folds
every feature's `di::register` (each self-gates on its `enabled` flag) into one `Wired` (merged
axum router + accumulating tonic router) → each feature nests its routes under `/api/v1` → the
`handler` binds the contract type, validates, calls `application::Service` → the use case parses
ids, applies business rules, maps domain↔contract → `port::Repository` (sqlx) behind an optional
`CachedRepository` decorator (catalog, Valkey cache-aside) → domain sentinel errors are mapped to
the HTTP envelope / gRPC status by `platform::errors::AppError`.

**Dependency gates** — `tests/architecture.rs` scans the `crate::`-rooted `use` statements of every
`src/**/*.rs` file (relative `super::`/sibling imports are intra-layer and out of scope) and
applies 9 rules; a violation fails with `module %q violates %s`. Go rule names are shown for
parity — use the **Rust rule names verbatim**. If a change trips a rule, **restructure the change
— never weaken the rule**:

| Go rule | Rust rule (`tests/architecture.rs`) | Forbids |
|---|---|---|
| `published-contract-is-outward-only` | `published-contract-is-outward-only` | internal code importing `crate::api` |
| `domain-is-innermost` | `domain-is-innermost` | `features/<f>/domain` depending on anything crate-internal |
| `contract-is-leaf` | `contract-is-leaf` | `features/<f>/contract` depending on anything crate-internal |
| `repository-interface-depends-only-on-domain` | `port-depends-only-on-domain` | `features/<f>/port` referencing anything but its own `domain` |
| `usecase-depends-on-domain-repository-contract` | `application-depends-on-domain-port-contract` | `features/<f>/application` importing anything outside its own domain/port/contract/application |
| `repository-impl-ignores-usecase-and-handler` | `driven-adapters-ignore-application` | `adapter/driven/**` importing `application` or `adapter/driving` |
| `handler-ignores-repository` | `driving-adapters-ignore-ports-and-driven-adapters` | `adapter/driving/**` importing `port` or `adapter/driven` |
| `features-registry-imports-only-own-features` | `features-registry-imports-only-own-features` | `features/mod.rs` + `features/registry.rs` reaching past a feature's module root or its `di` entry point into `domain`/`contract`/`port`/`application`/`adapter` |
| `infrastructure-ignores-features` | `platform-ignores-features` | `platform/**` importing `features/**` |

## Key Directories

- `src/main.rs` — thin entry point: loads config, validates, installs the signal handler, calls
  `zercle_rust_template::run_with_config`.
- `src/bin/migrate.rs` — self-contained migration runner (`up`/`down [N]`/`force`/`version`);
  builds one merged `sqlx::Migrator` from `features::registry::migrator()`.
- `src/app.rs` — composition root (`build`, `run`); the only place platform wiring order is
  defined. Feature wiring is delegated to `features::registry::register_all`.
- `src/features/registry.rs` — the feature registry: `list`, `register_all`, `migrator`; the
  single place features are enumerated. Its order is also the migration order (catalog 1,
  machines 2, sales 3).
- `src/features/{catalog,machines,sales,reporting}/` — the four demo features. Layers, each its
  own module: `domain` (entities + sentinels), `contract` (zero-dep wire types), `port` (outbound
  `Repository` trait), `application` (`Service` trait + `Usecase`), `adapter/driving` (axum +
  tonic), `adapter/driven/postgres` (sqlx impl, `migrations/`), `di`. Features never import each
  other; `sales` and `reporting` read other features' tables through their own repository ports.
- `src/api/` — published surface: `v1.rs` re-exports every feature's contract types, `errcodes.rs`
  re-exports the canonical codes. Outward-only.
- `src/platform/` — cross-cutting: `config`, `db`, `valkey`, `errcodes`, `errors`, `health`,
  `telemetry`, `middleware`, `server`.
- `tests/architecture.rs` — the dependency gates. `tests/e2e.rs` — end-to-end smoke test.
  `tests/common/mod.rs` — shared test helpers.
- `proto/<feature>/v1/<feature>.proto` — per-feature gRPC contracts, compiled by `build.rs`.

## Development Commands

[Task](https://taskfile.dev) is the runner — prefer `task <name>` over raw `cargo`:

- `task build` — release build of the `server` binary (version metadata via env → `option_env!`).
  `task build-all` builds both `server` and `migrate`.
- `task run` — run the release server (depends on `build`).
- `task test` (alias `task test-unit`) — unit suite (`cargo test --all-targets`).
- `task test-integration` — the full suite with `--include-ignored`; needs live postgres + Valkey
  (`docker compose up -d postgres valkey`, then `cp .env.example .env`).
- `task test-architecture` — the dependency gates only (`cargo test --test architecture`).
- `task test-e2e` — boots the full server (skips if the DB/Valkey TCP probe fails).
- `task lint` / `task fmt` / `task fmt-check` — clippy (`-D warnings`) / rustfmt / rustfmt check.
- `task tidy` / `task verify` — `cargo fetch` + `git diff --exit-code Cargo.lock` (tidy); tidy
  plus `git diff --exit-code Cargo.lock Cargo.toml` (verify). This is the `go mod tidy`/drift
  analogue.
- `task migrate-up` / `migrate-down [N=1]` — run the merged migration set via
  `cargo run --bin migrate`; `migrate-create FEATURE=<name> NAME=...` hand-writes an empty
  reversible pair with the next free global version into
  `src/features/<FEATURE>/adapter/driven/postgres/migrations` (`sqlx migrate add -r` is the noted
  per-directory alternative when sqlx-cli is installed).
- `task proto` — no-op; proto regeneration is handled by `build.rs` (tonic-build) on every
  `cargo build`.
- `task docker-build` / `docker-build-migrate` — build the server / migrate container image.
- `task cover` — `cargo-llvm-cov` coverage (lcov + HTML) with a 60% line gate.

`cargo test` runs the unit suites and the architecture gates; the live-infra suites are gated
behind `#[ignore]` and only run with `--include-ignored` (the Rust equivalent of Go's build tags):

```bash
cargo test --all-targets                                            # unit + architecture only
cargo test --test architecture                                      # dependency gates only
cargo test --all-targets -- --include-ignored --test-threads=1      # + live-infra integration
DATABASE_URL=postgres://postgres:postgres@localhost:5432/it_catalog \
  cargo test --lib catalog -- --include-ignored                     # one feature's integration suite
```

## Code Conventions & Common Patterns

- **Naming.** Layer modules are fixed lowercase nouns (`domain`, `contract`, `port`,
  `application`, `adapter/driving`, `adapter/driven`, `di`). Ports are named by role, not feature:
  `application::Service` (inbound), `port::Repository` (outbound). Impls: `Usecase`, `PgRepository`,
  `CachedRepository` (decorator), `Handler`, `GrpcServer`. Every feature exposes
  `di::register_with_grpc` (the registry entry point) and `di::register` (standalone test path).
- **Ports & mocks.** Traits use `#[cfg_attr(test, automock)]` (mockall) to generate
  `MockRepository` / `MockService`; use-case and adapter unit tests need no real DB.
- **Error handling.** Three tiers: (1) **domain sentinels** — a per-feature `domain::Error` enum
  (`thiserror`) in `domain/error.rs`; (2) **boundary error** — `AppError` in `platform/errors.rs`
  with `code()`/`message()`/`http_status()`/`grpc_code()`, codes from `platform::errcodes` so
  served and published codes cannot drift; (3) **registration** — each feature's `di` defines a
  crate-global `impl From<domain::Error> for AppError`, and handlers map any error via
  `AppError::from`. The HTTP envelope is always `{"error": code, "message": msg}`; the gRPC mapper
  redacts `Internal` causes from the wire message (CWE-209).
- **Config.** Loaded from `config.yaml` + unprefixed env vars (the `config` crate + `validator`);
  every leaf is explicitly bound in `platform/config.rs::leaf_bindings` (decision D5). `CONFIG_FILE`
  overrides the path, and `<exe_dir>/config.yaml` is a final fallback for the distroless image.
  `Config::load` + `validate` + `validate_cross` run at startup, so bad values fail fast.
  `CATALOG_ENABLED` / `MACHINES_ENABLED` / `SALES_ENABLED` / `REPORTING_ENABLED` gate each demo
  feature's providers and routes entirely.
- **Persistence.** The SQL schema is owned by `sqlx::migrate!` files under
  `adapter/driven/postgres/migrations/` (embedded at compile time). The sqlx query macros are
  **not** used (queries are runtime-checked `sqlx::query`), so there is **no `AutoMigrate`-equivalent
  and no offline `.sqlx` cache**: schema changes go through a migration file, never a runtime sync.
- **Generated code — regenerate, never hand-edit.** `build.rs` runs tonic-build over every
  `proto/*/v1/*.proto` (`build_server(true)`, `build_client(false)`); the generated modules live in
  `OUT_DIR` and are pulled in by `tonic::include_proto!("catalog.v1")` (package dots become
  underscores: `catalog_v1`) in each feature's `adapter/driving/grpc.rs`. `cargo build` regenerates.
- **`async-trait` ports.** `port::Repository`, `application::Service`, and `platform::health::Checker`
  are `#[async_trait]` traits carrying `#[allow(clippy::double_must_use)]`: the macro expansion
  wraps an already-`#[must_use]` future, which clippy flags on the macro span. The allow is
  deliberate and localized to the trait definition.
- **Lints.** `[lints.clippy] all = { level = "deny" }` in `Cargo.toml`, plus CI's
  `RUSTFLAGS: "-D warnings"`. Formatting is rustfmt (`rustfmt.toml`).

### Config leaf bindings (`src/platform/config.rs::leaf_bindings`)

Every leaf is bound explicitly (decision D5); env names match the Go template. Source of truth:
`src/platform/config.rs`.

```
app.name APP_NAME · app.environment APP_ENVIRONMENT · app.host APP_HOST · app.port APP_PORT
app.shutdown_timeout APP_SHUTDOWN_TIMEOUT
http.host HTTP_HOST · http.port HTTP_PORT · http.read_timeout HTTP_READ_TIMEOUT
http.write_timeout HTTP_WRITE_TIMEOUT · http.idle_timeout HTTP_IDLE_TIMEOUT
http.body_limit HTTP_BODY_LIMIT · http.health_probe_timeout HTTP_HEALTH_PROBE_TIMEOUT
http.cors_allow_origins HTTP_CORS_ALLOW_ORIGINS · http.cors_allow_methods HTTP_CORS_ALLOW_METHODS
http.cors_allow_headers HTTP_CORS_ALLOW_HEADERS
grpc.host GRPC_HOST · grpc.port GRPC_PORT
db.host DB_HOST · db.port DB_PORT · db.name DB_NAME · db.user DB_USER · db.password DB_PASSWORD
db.ssl_mode DB_SSL_MODE · db.max_conns DB_MAX_CONNS · db.min_conns DB_MIN_CONNS
db.max_conn_idle DB_MAX_CONN_IDLE · db.max_conn_life DB_MAX_CONN_LIFE
db.connect_timeout DB_CONNECT_TIMEOUT
valkey.host VALKEY_HOST · valkey.port VALKEY_PORT · valkey.password VALKEY_PASSWORD
valkey.db VALKEY_DB · valkey.connect_timeout VALKEY_CONNECT_TIMEOUT · valkey.ttl VALKEY_TTL
log.level LOG_LEVEL · log.format LOG_FORMAT
otel.exporter OTEL_EXPORTER · otel.endpoint OTEL_EXPORTER_OTLP_ENDPOINT
otel.service_name OTEL_SERVICE_NAME · otel.sampling OTEL_TRACES_SAMPLER_ARG
catalog.enabled CATALOG_ENABLED · catalog.default_page_size CATALOG_DEFAULT_PAGE_SIZE
catalog.max_page_size CATALOG_MAX_PAGE_SIZE · catalog.max_name_length CATALOG_MAX_NAME_LENGTH
machines.enabled MACHINES_ENABLED · machines.default_page_size MACHINES_DEFAULT_PAGE_SIZE
machines.max_page_size MACHINES_MAX_PAGE_SIZE · machines.max_label_length MACHINES_MAX_LABEL_LENGTH
sales.enabled SALES_ENABLED
reporting.enabled REPORTING_ENABLED · reporting.default_top_machines REPORTING_DEFAULT_TOP_MACHINES
reporting.max_top_machines REPORTING_MAX_TOP_MACHINES
```

`DATABASE_URL` is **not** a config leaf: server/migrate binaries build their connection string
from the `DB_*` leaves, while the integration test harnesses read `DATABASE_URL` directly (see
Testing & QA).

## Important Files

- `src/app.rs` — composition root: platform wiring order, `build`, `run`.
- `src/features/registry.rs` — the registry (`list`, `register_all`, `migrator`); source of truth
  for feature enumeration and the migration namespace.
- `tests/architecture.rs` — the 9 dependency gates (source of truth for layering).
- `src/features/catalog/di.rs` — the canonical feature wiring: flag gate, sentinel mapping,
  repository (+ cache-aside decorator), providers, route mount.
- `src/features/catalog/adapter/driven/cached.rs` — the cache-aside decorator (key, TTL,
  not-found-never-cached).
- `src/platform/config.rs` — config struct, per-leaf env binding, `validate` / `validate_cross`.
- `src/platform/errors.rs` — `AppError` + `errcodes` re-export + `IntoResponse` + gRPC mapper.
- `src/platform/server/` — `mod.rs` (`AppState`, `run`, ordered shutdown), `http.rs` (router +
  middleware + shared routes), `grpc_interceptor.rs`, `shutdown.rs`.
- `src/bin/migrate.rs` — the migration runner over `registry::migrator()`.
- `Taskfile.yml`, `config.yaml`, `.env.example`, `compose.yml`, `Containerfile`,
  `Containerfile.migrate`, `.github/workflows/ci.yml`.

## Runtime/Tooling Preferences

- **Rust** stable, pinned by `rust-toolchain.toml` (components `rustfmt`, `clippy`); the crate is
  edition 2024 with `rust-version = 1.85` in `Cargo.toml`. CI uses `dtolnay/rust-toolchain`.
- **protoc** is required by `build.rs` (tonic-build has no bundled compiler); CI installs it via
  `arduino/setup-protoc`. The Containerfiles install `protobuf-compiler`.
- Key deps: `axum` 0.8, `tonic` 0.12 + `prost` 0.13, `sqlx` 0.8 (runtime-tokio-rustls, postgres,
  uuid, time, macros, migrate), `redis` 0.27 (tokio-rustls-comp, connection-manager), `config` 0.14,
  `validator` 0.18, `tracing` + `tracing-subscriber` + `opentelemetry*` + `opentelemetry-prometheus`,
  `thiserror`, `anyhow`, `uuid`, `time`, `humantime`. Dev: `mockall`, `pretty_assertions`, `reqwest`,
  `tower`. Build: `tonic-build`.
- Local services: `docker compose up -d postgres valkey` (`compose.yml` uses `postgres:18-alpine`
  and `valkey/valkey:9-alpine`). `Containerfile` is a two-stage musl-static →
  `distroless/static` non-root server image; `Containerfile.migrate` mirrors it for the `migrate`
  binary.

## Testing & QA

- **Tiers.** Unit tests are hermetic (mockall mocks, no infra) and live in-module under
  `#[cfg(test)]`; they run with `cargo test --all-targets`. Integration tests are live-infra
  suites inside `#[cfg(test)] mod integration` (catalog's in `adapter/integration.rs`, sales' and
  reporting's at the `di` composition edge) gated by `#[ignore]`. The e2e test is
  `tests/e2e.rs`.
- **Ignore-gating = Go build tags.** Every live-infra test is `#[ignore]`; plain `cargo test`
  compiles but does not run it. `--include-ignored` runs the full live suite (CI integration job).
- **Integration setup.** No testcontainers: the suites read `DATABASE_URL` (a local throwaway
  database), apply the registry's merged migrations in-suite, and `TRUNCATE` between cases. They
  **hard-fail** (never skip) when infra is missing, and a **production guard** panics on
  `APP_ENVIRONMENT=production` or a non-local `DATABASE_URL`/`VALKEY_HOST`. They serialize on a
  shared advisory lock / `Mutex` because they share one database.
- **e2e.** `tests/e2e.rs` boots the full app on ephemeral ports and probes the documented routes;
  it self-skips (returns `Ok`) when the DB/Valkey TCP probe fails, so a partial local setup never
  breaks `cargo test`.
- **Architecture gate.** `cargo test --test architecture` — run after any refactor that moves
  imports or modules.
- **Coverage.** 60% line gate, enforced in CI (`ci.yml` unit job) via
  `cargo llvm-cov report --fail-under-lines`; not in the Taskfile. Locally use `task cover` or
  `cargo llvm-cov --workspace --all-targets`.
- **CI.** `.github/workflows/ci.yml`: `fmt` → `clippy` → `architecture` + `unit` (60% gate,
  codecov, coverage artifact) → `integration` (postgres + Valkey service containers, migrations
  via the `migrate` binary) → `build` (both binaries + `docker build`). `security.yml` runs a
  weekly Trivy scan + `cargo-audit`; `cd.yml` publishes multi-arch server **and** migrate images on
  a `v*` tag.

## Gotchas

- **`clippy::double_must_use` on async-trait ports.** The `#[allow]` on `port::Repository` /
  `application::Service` / `health::Checker` is required, not a mistake — `async-trait` 0.1.89
  expands the async method to an already-`#[must_use]` future, which clippy flags on the macro
  span. Do not "fix" it by removing the trait's async.
- **Generated proto lives in `OUT_DIR`.** Each feature's `adapter/driving/grpc.rs` calls
  `tonic::include_proto!("<package>")`; the package dots become underscores (`catalog.v1` →
  `catalog_v1`). Never hand-edit generated code — change the `.proto` and rebuild.
- **Feature gating defaults to `false`.** `CatalogConfig`/`MachinesConfig`/`SalesConfig`/
  `ReportingConfig` default `enabled = false` (Go `setDefaults` parity), while `config.yaml` sets
  all four to `true`. A config file that omits a feature section registers nothing.
- **Stale-checkout lockfile drift.** `task tidy` runs `cargo fetch` then
  `git diff --exit-code Cargo.lock`; a stale checkout whose `Cargo.toml` changed without a
  regenerated `Cargo.lock` fails — the Rust analogue of Go's `go.mod`/`go.sum` drift check.
- **No global dotenv.** The Taskfile does not use dotenv; `Config::load` overlays env on top of
  `config.yaml`. Adding a global dotenv would leak `.env` values into runs that expect the yaml
  defaults (the unit config tests build from strings, but e2e/integration read the real env).
- **The four demo features are replaceable.** Adding a feature is its directory plus one
  `registry::list()` entry; replacing them is the reverse (feature dirs, `registry::list()`
  entries, `api::v1` re-exports, `config.yaml`/`.env.example`, `Config`/`leaf_bindings`, and the
  `Taskfile.yml` migrate `FEATURE=`).
