# zercle-rust-template

Opinionated Rust microservice template — axum (HTTP) + tonic (gRPC) + sqlx (PostgreSQL) +
`redis` (Valkey) + `tracing`/`opentelemetry`, organized as clean-architecture-per-feature with a
single composition root (`Arc<AppState>`).

Faithful Rust port of [zercle-go-template](../zercle-go-template). See
`.agents/plans/rust-template-port/canvas.md` and `structure.md` for the full spec.

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

# 2. Apply migrations (wave 5 — `migrate` binary is a stub in wave 1)
psql -h localhost -U postgres -d app -f migrations/000001_create_items_table.up.sql

# 3. Run the server (skeleton in wave 1 — see below)
cargo run --bin server
# or, with task installed:
task run
```

`cargo run --bin server` reads `config.yaml` from the working directory and overlays environment
variables using the explicit leaf-binding table — env names match the Go template exactly
(`APP_NAME`, `DB_HOST`, `VALKEY_PASSWORD`, …).

## Directory tree

```
zercle-rust-template/
├── .agents/                                  # harness state (conductor-owned)
├── .github/
│   └── dependabot.yml                        # weekly cargo + actions + docker updates
├── proto/
│   └── example/v1/example.proto              # stub feature gRPC contract
├── migrations/
│   ├── 000001_create_items_table.up.sql
│   └── 000001_create_items_table.down.sql
├── src/
│   ├── main.rs                               # bin `server`: load config → lib::run
│   ├── lib.rs                                # crate root + module declarations
│   ├── bin/migrate.rs                        # bin `migrate` (skeleton — wave 5)
│   ├── config.rs                             # Config + Load + Validate (decision D5)
│   ├── app.rs                                # AppState + build() + run() (composition root)
│   ├── shared/
│   │   ├── errors.rs                         # AppError + IntoResponse + tonic mapper
│   │   ├── health.rs                         # Checker trait + Registry
│   │   ├── telemetry.rs                      # tracing + OTel + Prometheus init
│   │   └── server/
│   │       ├── mod.rs                        # Application orchestrator
│   │       ├── http.rs                       # axum router, middleware stack
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
│       └── example/                          # STUB FEATURE — delete to start
│           ├── mod.rs
│           ├── domain.rs                     # Item entity + Repository/Service traits
│           ├── dto.rs                        # request / response shapes
│           ├── repository.rs                 # sqlx impl of Repository
│           ├── service.rs                    # use-case impl of Service
│           ├── handler.rs                    # axum HTTP handlers
│           └── grpc.rs                       # tonic ExampleService server
├── tests/                                    # integration + e2e (added in wave 7)
├── build.rs                                  # tonic-build: compile proto/example/v1/example.proto
├── Cargo.toml                                # crate manifest
├── Cargo.lock                                # committed for reproducible builds
├── rust-toolchain.toml                       # pinned stable + rustfmt + clippy
├── rustfmt.toml
├── Taskfile.yml                              # cargo wrapper (wave 6)
├── Containerfile                             # multi-stage musl + distroless (wave 6)
├── Containerfile.migrate                     # migrate image (wave 6)
├── compose.yml                               # postgres + valkey + observability profile
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

## Removing the stub feature

`src/features/example/` ships with `//! STUB FEATURE — delete src/features/example to start your
project.` headers — it's a worked example of the clean-architecture layout. To start a real
project:

```bash
rm -rf src/features/example
```

Then edit `src/lib.rs` to drop `pub mod features;` (or keep `features` and add your own
sub-module) and update `Cargo.toml` (drop tonic-build if you don't need gRPC).

## Testing

```bash
cargo test                 # unit tests (config, errors, health — all green in wave 1)
cargo test --features integration  # integration tests (postgres + valkey required)
```

The wave 1 skeleton includes unit tests for `config` (yaml parse + validate), `errors` (status /
code mapping), and `health` (registry semantics). Integration and e2e tests land in wave 7.

## Deployment

- **Local containers**: `docker compose up -d` (add `--profile observability` for OTel +
  Prometheus + Grafana).
- **Kubernetes**: `kubectl apply -k deployments/kustomize/overlays/development`. The base
  `Deployment` runs the distroless image as non-root with `readOnlyRootFilesystem: true`;
  secrets hold `DB_PASSWORD` / `VALKEY_PASSWORD`.
- **Container build**: `docker build -f Containerfile -t zercle-rust-template .` (multi-stage
  `rust:slim` builder → distroless/static non-root final; wave 6).

## Migration from `zercle-go-template`

- All env var names are identical (`APP_NAME`, `DB_HOST`, `OTEL_TRACES_SAMPLER_ARG`, …) — point
  your existing config maps / secrets at the new workload without changes.
- All config keys are identical — `config.yaml` round-trips.
- gRPC service definitions are byte-faithful (`proto/example/v1/example.proto`); existing
  protobuf clients work unchanged.
- Health endpoints (`/healthz`, `/readyz`) and metrics path (`/metrics`) match.

## License

MIT — see `LICENSE`.
