# zercle-rust-template

Opinionated Rust microservice template — axum (HTTP) + tonic (gRPC) + sqlx (PostgreSQL) +
`redis` (Valkey) + `tracing`/`opentelemetry`, organized as clean-architecture-per-feature with a
single composition root (`Arc<AppState>`).

## Prerequisites

- **Rust** stable (toolchain pinned via `rust-toolchain.toml`; edition 2024)
- **protoc** 3.15+ (for the proto compile in `build.rs` — Rust gRPC generation)
- **PostgreSQL 18+** and **Valkey 9+** (or use `compose.yml`)
- **Docker / Podman** for local infra (optional — `cargo` works directly against the host)
- **Task** (`go-task`) — optional wrapper around `cargo` commands; install from
  <https://taskfile.dev>. If absent, run the `cargo` commands directly.

## Quick start

```bash
# 1. Start local infra (postgres + valkey; add `observability` profile for OTel + Prometheus + Grafana)
docker compose up -d postgres valkey

# 2. Apply migrations via the `migrate` binary (reads DATABASE_URL or DB_* env vars)
cargo run --bin migrate -- up
# or, with task installed:
task migrate-up

# 3. Run the server
cargo run --bin server
# or, with task installed:
task run
```

`cargo run --bin server` reads `config.yaml` from the working directory and overlays environment
variables using the explicit leaf-binding table — env names match the Go template exactly
(`APP_NAME`, `DB_HOST`, `VALKEY_PASSWORD`, …).

For the full containerised stack (migrations as a one-shot `migrate` service, then `server` plus
optional `observability` profile):

```bash
docker compose up -d                       # postgres + valkey + migrate + server
docker compose --profile observability up -d  # + otel-collector, prometheus, grafana
```

## Migrate subcommands

The `migrate` binary supports the standard sqlx-style subcommands:

| Subcommand            | Description                                                  |
| --------------------- | ------------------------------------------------------------ |
| `up`                  | Apply all pending migrations.                                |
| `down [N]`            | Roll back the most recent `N` migrations (default `1`).      |
| `force VERSION`       | Force-set the migration version table (recovery / repair).  |
| `version`             | Print the current applied migration version.                |

## Directory tree

```
zercle-rust-template/
├── .agents/                                  # harness state (conductor-owned)
├── .github/
│   ├── dependabot.yml                        # weekly cargo + actions + docker updates
│   └── workflows/
├── proto/
│   └── example/v1/example.proto              # example feature gRPC contract
├── migrations/
│   ├── 000001_create_items_table.up.sql
│   └── 000001_create_items_table.down.sql
├── src/
│   ├── main.rs                               # bin `server`: load config → lib::run
│   ├── lib.rs                                # crate root + module declarations
│   ├── bin/migrate.rs                        # bin `migrate`: up / down [N] / force / version
│   ├── config.rs                             # Config + Load + Validate (decision D5)
│   ├── app.rs                                # AppState + build() + run() (composition root)
│   ├── shared/
│   │   ├── errors.rs                         # AppError + IntoResponse + tonic mapper
│   │   ├── health.rs                         # Checker trait + Registry
│   │   ├── telemetry.rs                      # tracing + OTel + Prometheus init
│   │   └── server/
│   │       ├── mod.rs                        # Application orchestrator
│   │       ├── http.rs                       # axum router, middleware stack
│   │       ├── grpc_interceptor.rs           # tonic unary interceptor (request_id + access log)
│   │       └── shutdown.rs                   # ordered graceful shutdown
│   ├── middleware/
│   │   ├── request_id.rs                     # X-Request-ID propagate / generate
│   │   ├── access_log.rs                     # one structured access log per request
│   │   ├── recover.rs                        # panic → 500
│   │   └── cors.rs                           # tower-http CORS from config
│   ├── infrastructure/
│   │   ├── db.rs                             # PgPool + ping + readiness checker
│   │   └── valkey.rs                         # redis client + ping + readiness checker
│   └── features/
│       └── example/                          # STUB FEATURE — delete to start your project
│           ├── mod.rs
│           ├── domain.rs                     # Item entity + Repository/Service traits
│           ├── dto.rs                        # request / response shapes
│           ├── repository.rs                 # sqlx impl of Repository
│           ├── service.rs                    # use-case impl of Service
│           ├── handler.rs                    # axum HTTP handlers
│           └── grpc.rs                       # tonic ExampleService server
├── tests/
│   ├── common/mod.rs                         # shared helpers for integration + e2e tests
│   ├── example_http.rs                       # integration: HTTP feature flows (--ignored)
│   └── e2e.rs                                # e2e: boots the full app (--ignored)
├── build.rs                                  # tonic-build: compile proto/example/v1/example.proto
├── Cargo.toml                                # crate manifest
├── Cargo.lock                                # committed for reproducible builds
├── rust-toolchain.toml                       # pinned stable + rustfmt + clippy
├── rustfmt.toml
├── Taskfile.yml                              # cargo wrapper (build, test, migrate, docker-build, cover)
├── Containerfile                             # multi-stage musl + distroless (server image)
├── Containerfile.migrate                     # multi-stage migrate image
├── compose.yml                               # postgres + valkey + migrate + server + observability profile
├── config.yaml                               # server config (same keys as Go template)
├── .env.example
├── deployments/
│   ├── kustomize/{base,overlays/development}/
│   └── observability/{otel-collector-config.yaml,prometheus.yml}
└── LICENSE
```

## Architecture overview

- **Composition root = `Arc<AppState>`** (no runtime DI container — idiomatic Rust; see
  `canvas.md ## Assumptions` row 9 / decision D2). `app::build` constructs every dependency
  (config, telemetry, `PgPool`, `ConnectionManager`, health registry, feature services), wraps
  the result in `Arc`, and hands it to axum `State` and tonic request extensions.
- **Clean architecture per feature**: `domain` (entities + `Repository`/`Service` traits + error
  enum) → `repository` (sqlx adapter) → `service` (use-case impl) → `handler` (axum) /
  `grpc.rs` (tonic) → `mod.rs` (`router()` + `grpc_service()`).
- **Trait-based ports + mockall mocks**: handlers and tests inject `MockRepository` /
  `MockService`; no real DB required for unit tests.
- **gRPC unary interceptor** (`src/shared/server/grpc_interceptor.rs`) mirrors the HTTP
  middleware stack's panic-recovery + access-log guarantees — it recovers handler panics
  into `tonic::Status::internal` and emits one structured access log per unary call.
  OTel tracing for gRPC (including streams) is provided by `Server::trace_fn`.
- **Typed errors**: each feature defines a `domain::Error` enum (`thiserror`) and a
  `From<domain::Error> for AppError` impl registered in the feature's `mod.rs`. The shared
  `AppError` enum maps to both `StatusCode` (axum) and `tonic::Code` at the boundary — no
  string matching.
- **Env binding = explicit leaf table** (decision D5): the `config` crate's default `_`
  separator would collide with SCREAMING_SNAKE names, so we port the Go `leafBindings()` table
  verbatim and override each leaf from `std::env` after loading the yaml.
- **Graceful shutdown**: SIGTERM/SIGINT triggers `axum::serve(...).with_graceful_shutdown` →
  tonic `Server::shutdown` → `PgPool::close` → `ConnectionManager` drop → OTel provider flush,
  all bounded by `cfg.app.shutdown_timeout`.

## Removing the example feature

`src/features/example/` ships with `//! STUB FEATURE — delete src/features/example to start
your project.` headers — it's a worked example of the clean-architecture layout. To start a
real project:

```bash
rm -rf src/features/example
```

Then edit `src/lib.rs` to drop `pub mod features;` (or keep `features` and add your own
sub-module) and update `Cargo.toml` (drop tonic-build if you don't need gRPC).

## Testing

```bash
# Unit tests (no infra needed)
cargo test --all-targets
# or: task test-unit

# Integration tests (postgres + valkey required; skip cleanly if unreachable)
docker compose up -d postgres valkey
cargo test --all-targets -- --ignored --test-threads=1
# or: task test-integration

# End-to-end test (boots the full app; needs infra + migrations applied)
task migrate-up
cargo test --test e2e -- --ignored
# or: task test-e2e
```

The unit suite covers `config` (yaml parse + validate), `errors` (status / code mapping),
`health` (registry semantics), and the `migrate` CLI parser. Both integration (`tests/example_http.rs`)
and e2e (`tests/e2e.rs`) tests are gated behind `--ignored` and skip cleanly when the relevant
infrastructure is unreachable, so a partial local setup never breaks `cargo test`.

Other quality gates:

```bash
cargo clippy --all-targets -- -D warnings   # or: task lint
cargo fmt --all -- --check                  # or: task fmt-check
cargo llvm-cov --workspace --all-targets    # or: task cover   (requires cargo-llvm-cov)
```

## Deployment

- **Local containers**: `docker compose up -d` (add `--profile observability` for OTel +
  Prometheus + Grafana). The compose stack runs the `migrate` service once before the
  `server` service starts.
- **Kubernetes**: `kubectl apply -k deployments/kustomize/overlays/development`. The base
  `Deployment` runs the distroless image as non-root with `readOnlyRootFilesystem: true`;
  secrets hold `DB_PASSWORD` / `VALKEY_PASSWORD`.
- **Container build**:
  - Server: `task docker-build` (`docker build -f Containerfile -t zercle-rust-template:latest .`)
  - Migrate: `task docker-build-migrate` (`docker build -f Containerfile.migrate …`)
  Both are multi-stage (`rust:slim` builder → distroless/static non-root final).

## Migration from `zercle-go-template`

- All env var names are identical (`APP_NAME`, `DB_HOST`, `OTEL_TRACES_SAMPLER_ARG`, …) — point
  your existing config maps / secrets at the new workload without changes.
- All config keys are identical — `config.yaml` round-trips.
- gRPC service definitions are byte-faithful (`proto/example/v1/example.proto`); existing
  protobuf clients work unchanged.
- Health endpoints (`/healthz`, `/readyz`) and metrics path (`/metrics`) match.

## License

MIT — see `LICENSE`.
