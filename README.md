# zercle-rust-template

Opinionated Rust microservice template — axum (HTTP) + tonic (gRPC) + sqlx (PostgreSQL) +
`redis` (Valkey) + `tracing`/`opentelemetry`, organized as clean architecture (DDD) per feature:
`contract → domain → port → application → adapter/{driving,driven}` with per-feature `di`
wiring, a decoupled platform shell, and a published inbound contract facade (`crate::api::v1`).
All dependencies point inward, and the rule is enforced executably in CI.

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
│   └── workflows/ci.yml                      # fmt → clippy → architecture → unit → integration → build
├── proto/
│   └── example/v1/example.proto              # example feature gRPC contract
├── migrations/
│   ├── 000001_create_items_table.up.sql
│   └── 000001_create_items_table.down.sql
├── src/
│   ├── main.rs                               # bin `server`: load config → lib::run
│   ├── lib.rs                                # crate root + module declarations
│   ├── bin/migrate.rs                        # bin `migrate`: up / down [N] / force / version
│   ├── app.rs                                # composition root: platform + features::example::di
│   ├── api/
│   │   ├── mod.rs                            # published facade namespace (outward-only)
│   │   └── v1.rs                             # re-exports contract types + errcodes (Go pkg/api/v1)
│   ├── platform/                             # cross-cutting shell — never imports features
│   │   ├── config.rs                         # Config + Load + Validate (decision D5)
│   │   ├── db.rs                             # PgPool + ping + readiness checker
│   │   ├── valkey.rs                         # redis client + ping + readiness checker
│   │   ├── errors.rs                         # AppError + errcodes + IntoResponse + tonic mapper
│   │   ├── health.rs                         # Checker trait + Registry
│   │   ├── telemetry.rs                      # tracing + OTel + Prometheus init
│   │   ├── middleware/
│   │   │   ├── request_id.rs                 # X-Request-ID propagate / generate
│   │   │   ├── access_log.rs                 # one structured access log per request
│   │   │   ├── recover.rs                    # panic → 500
│   │   │   └── cors.rs                       # tower-http CORS from config
│   │   └── server/
│   │       ├── mod.rs                        # AppState + run() + grpc_server() builder
│   │       ├── http.rs                       # axum router, middleware stack, shared routes
│   │       ├── grpc_interceptor.rs           # tonic unary interceptor (request_id + access log)
│   │       └── shutdown.rs                   # ordered graceful shutdown
│   └── features/
│       └── example/                          # STUB FEATURE — delete to start your project
│           ├── contract/                     # inbound wire types (LEAF; published via crate::api::v1)
│           │   ├── create_item.rs            # CreateItemRequest + ItemResponse
│           │   └── list_items.rs             # ListItemsRequest + ListItemsResponse
│           ├── domain/                       # innermost layer (no crate deps)
│           │   ├── item.rs                   # Item entity
│           │   └── error.rs                  # domain sentinel errors
│           ├── port/
│           │   └── repository.rs             # outbound Repository trait (mockall)
│           ├── application/
│           │   ├── service.rs                # inbound Service trait — speaks contract types
│           │   └── usecase.rs                # Usecase impl: domain ↔ contract mapping
│           ├── adapter/
│           │   ├── driving/                  # driving adapters (call application::Service)
│           │   │   ├── http.rs               # axum handlers
│           │   │   └── grpc.rs               # tonic ExampleService server
│           │   └── driven/
│           │       └── postgres.rs           # sqlx impl of port::Repository
│           ├── di.rs                         # wiring + sentinel→AppError registration
│           └── mod.rs                        # layer map + delete-me notice
├── tests/
│   ├── common/mod.rs                         # shared helpers for integration + e2e tests
│   ├── architecture.rs                       # executable dependency gates (layering rules)
│   ├── example_http.rs                       # integration: feature flows via di (self-skips w/o infra)
│   └── e2e.rs                                # e2e: boots the full app (self-skips w/o infra)
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

Clean architecture (DDD) per feature, mirroring `zercle-go-template`'s
`internal/features/<name>/{contract,domain,port,application,adapter,di}`. **All dependencies
point inward** — and the rule is executable, not aspirational: `tests/architecture.rs`
(the CI **Architecture** job) scans every `use crate::…` statement and fails on any
layer violation.

- **`contract/` — the exposed inbound type contract (leaf)**: canonical request/response wire
  types (`serde` + `validator` only). Both driving adapters bind these directly; the published
  facade `crate::api::v1` re-exports them (plus `errcodes`) so *other services* can construct
  payloads without importing server internals. Internal code may never import `crate::api`.
- **`domain/` — innermost**: entities + sentinel errors, no crate-internal dependencies.
- **`port/` — outbound (driven) ports**: the `Repository` trait; references only its own domain.
- **`application/` — use cases**: the inbound `Service` port speaks **contract types** at the
  boundary (`create(CreateItemRequest) -> ItemResponse`), so driving adapters never map to or
  from domain entities. The `Usecase` impl owns the domain ↔ contract mapping and the single
  validation path (e.g. wire-id parsing) shared by HTTP and gRPC.
- **`adapter/driving/`** (axum handlers, tonic server): translate transport ↔ contract, call
  `application::Service`. **`adapter/driven/`** (sqlx): implements `port::Repository` —
  (`in` is a Rust keyword, hence the hexagonal `driving`/`driven` names).
- **`di.rs` — composition edge**: wires repository → use case → adapters, nests HTTP routes
  under `/api/v1`, registers the feature's gRPC service on the platform's tonic builder, and
  registers the `domain::Error → AppError` sentinel mapping (Go `RegisterSentinel` parity).
- **`platform/` — cross-cutting shell, feature-agnostic by rule** (`platform-ignores-features`
  gate): config, db, valkey, boundary errors (`AppError` + `errcodes`), health, telemetry,
  middleware, and the HTTP/gRPC server shell. Feature routers arrive pre-mounted; the shell
  adds shared routes (`/healthz`, `/readyz`, `/metrics`) and the middleware stack.
- **`app.rs` — composition root**: builds platform in dependency order (telemetry → postgres →
  valkey → health) and calls each feature's `di::register` — the only feature symbol the shell
  references. Adding a feature = adding one `di::register` call.
- **Trait-based ports + mockall mocks**: `#[cfg_attr(test, automock)]` generates
  `MockRepository` / `MockService`; use-case and adapter unit tests need no real DB.
- **gRPC unary interceptor** (`src/platform/server/grpc_interceptor.rs`) mirrors the HTTP
  middleware stack's panic-recovery + access-log guarantees — it recovers handler panics
  into `tonic::Status::internal` and emits one structured access log per unary call.
  OTel tracing for gRPC (including streams) is provided by `Server::trace_fn`.
- **Typed errors**: each feature's `domain::Error` sentinels map to the shared `AppError`
  at the `di` composition edge; `AppError` maps to both `StatusCode` (axum) and
  `tonic::Code` at the boundary — no string matching.
- **Env binding = explicit leaf table** (decision D5): the `config` crate's default `_`
  separator would collide with SCREAMING_SNAKE names, so we port the Go `leafBindings()` table
  verbatim and override each leaf from `std::env` after loading the yaml.
- **Graceful shutdown**: SIGTERM/SIGINT triggers `axum::serve(...).with_graceful_shutdown` →
  tonic `Router::serve_with_shutdown` drain → `PgPool::close` → `ConnectionManager` drop →
  OTel provider flush, all bounded by `cfg.app.shutdown_timeout`.

## Removing the example feature

`src/features/example/` ships with `//! STUB FEATURE — delete src/features/example to start
your project.` headers — it's a worked example of the clean-architecture layout. To start a
real project:

```bash
rm -rf src/features/example
```

Then:

1. Drop `pub mod example;` from `src/features/mod.rs` (or keep `features` and add your own
   sub-module).
2. Remove the `example::di::register` call from `src/app.rs` and the `example` section from
   `config.yaml` / `Config` (`src/platform/config.rs`).
3. Remove the published facade re-exports in `src/api/v1.rs` (or point them at your new
   feature's `contract` module) and drop `tonic-build` from `Cargo.toml` if you don't need gRPC.

The platform, `api` facade pattern, and `app` shell do not reference the feature from anywhere
else — that is what the architecture test enforces.

## Testing

```bash
# Unit tests (no infra needed)
cargo test --all-targets
# or: task test-unit

# Clean-architecture dependency gates (layering rules; no infra needed)
cargo test --test architecture
# or: task test-architecture

# Full suite against live postgres + valkey (skips cleanly if unreachable)
docker compose up -d postgres valkey
cargo test --all-targets -- --include-ignored --test-threads=1
# or: task test-integration

# End-to-end test (boots the full app; needs infra + migrations applied)
task migrate-up
cargo test --test e2e
# or: task test-e2e
```

The unit suite covers `config` (yaml parse + validate), `errors` (status / code mapping),
`health` (registry semantics), `contract` (wire shapes + validation), `application`
(use-case rules via mocks), and both driving adapters. `tests/architecture.rs` enforces the
layering. The integration (`tests/example_http.rs`) and e2e (`tests/e2e.rs`) tests skip
cleanly when the relevant infrastructure is unreachable, so a partial local setup never
breaks `cargo test`.

Other quality gates:

```bash
cargo clippy --all-targets -- -D warnings   # or: task lint
cargo fmt --all -- --check                  # or: task fmt-check
cargo llvm-cov --workspace --all-targets    # or: task cover   (requires cargo-llvm-cov)
```

## CI (GitHub Actions)

`.github/workflows/ci.yml` runs on every push/PR to `main`/`develop`:

1. **fmt** — `cargo fmt --all -- --check`.
2. **clippy** — `cargo clippy --all-targets --locked -- -D warnings`.
3. **architecture** — the layering gates from `tests/architecture.rs` (fast, dedicated signal).
4. **unit** — full test run with `cargo-llvm-cov` coverage; lcov + HTML artifacts uploaded,
   gated at 60% line coverage.
5. **integration** — full suite (`--include-ignored`) against real `postgres:18-alpine` +
   `valkey:9-alpine` service containers, migrations applied via the `migrate` binary.
6. **build** — release build of both binaries (version metadata injected) + a
   `docker build` of the Containerfile.

All actions are pinned to commit SHAs and checkouts run with `persist-credentials: false`;
Dependabot keeps the pins, crates, and base images updated weekly.

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
