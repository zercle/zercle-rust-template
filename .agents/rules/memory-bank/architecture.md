# System Architecture

## Architectural Style
Clean Architecture with Domain-Driven Design (DDD) principles, adapted for Rust with traits, async/await, and ownership patterns.

## Layer Structure

### 1. Domain Layer (`src/domain/`)
**Purpose:** Core business logic and entities, independent of infrastructure.

**Components per Domain:**
- `entity.rs` - Business entities with domain logic
- `mod.rs` - Domain module exports and interfaces (traits)
- `repository.rs` - Repository implementations
- `service.rs` - Business logic and orchestration (usecase)
- `handler.rs` - HTTP request handlers (Axum handlers)
- `request.rs` - DTOs for incoming requests
- `response.rs` - DTOs for outgoing responses
- `mock.rs` - Mock implementations for testing

**Current Domains:**
- `user/` - User authentication and profile management
- `task/` - Task management (example domain)

### 2. Infrastructure Layer (`src/infrastructure/`)
**Purpose:** External concerns and technical implementations.

**Sub-modules:**
- `config/` - Configuration management with config crate
- `db/` - Database abstraction and factory
- `http/client/` - HTTP client (reqwest wrapper)
- `logger/` - Structured logging (tracing wrapper)
- `password/` - Password hashing (argon2 crate)

### 3. Application Layer (`src/app/`)
**Purpose:** Application orchestration and dependency injection.

**Key Components:**
- `mod.rs` - Main application module
- `state.rs` - Application state for Axum
- Dependency wiring with Arc and traits
- Middleware setup with Tower layers
- Route registration with Axum Router
- Server lifecycle management with Tokio

### 4. Entry Point (`src/main.rs`)
**Purpose:** Application bootstrap.

**Components:**
- `main.rs` - Application initialization and startup
- Tokio runtime setup
- Axum server configuration
- Graceful shutdown handling

## Component Boundaries

### Domain Boundaries
- Each domain is self-contained with its own traits
- Domains communicate through well-defined trait boundaries
- No direct dependencies between domains
- Shared infrastructure through traits only

### Infrastructure Boundaries
- Infrastructure implements domain traits
- Domain layer depends on abstractions (traits), not implementations
- Database access abstracted through repository trait pattern
- External services abstracted through client traits

### Rust-Specific Boundaries
- Use `Arc<T>` for shared state across handlers
- Use `Send + Sync` bounds for thread-safe shared state
- Use `async fn` for all I/O operations
- Use `Result<T, E>` for error propagation
- Use `?` operator for error handling

## Data Flow

### Request Flow
```
Client → Handler → Service → Repository → Database
          ↓         ↓          ↓
      Request   Business    Data Access
      DTO       Logic       Layer
          ↓         ↓          ↓
     Response  Entity    sqlx Query
     DTO       Mapping    Generation
```

### Async Flow
- All handlers are async functions
- All database operations use sqlx async queries
- All external HTTP calls use reqwest async client
- Tokio runtime manages async tasks

### Authentication Flow
1. User submits credentials to `/api/v1/auth/login`
2. Handler validates request DTO with serde
3. Service retrieves user by email
4. Password verified using argon2
5. JWT token generated with user ID and email
6. Token returned in response

### Protected Route Flow
1. Request includes JWT in Authorization header
2. JWT middleware validates token (Tower layer)
3. User ID extracted from token claims
4. Handler processes request with user context
5. Service enforces ownership rules

## Module Interactions

### Dependency Injection
- Application layer creates all dependencies
- Dependencies passed through constructors
- Traits used for all external dependencies
- Mock implementations for testing
- Use `Arc<T>` for shared dependencies

### Database Access Patterns
- sqlx provides type-safe async queries
- Repository trait pattern abstracts database
- Transactions managed at repository level with `.begin()`
- Connection pooling handled by sqlx PgPool
- Use `PgPool` for connection management

### Error Handling Strategy
- Domain-specific errors in service layer with `thiserror`
- Repository errors wrapped with context
- HTTP status codes mapped in handler layer
- Structured error responses to clients
- Use `Result<T, E>` throughout the stack

### Ownership and Borrowing
- Use `&self` for read-only operations
- Use `&mut self` for write operations
- Use `Arc<T>` for shared state across async tasks
- Use `Clone` for cheap-to-clone types
- Use `Cow<T>` for borrowed or owned data

## Integration Points

### External Services
- PostgreSQL database (primary data store) via sqlx
- Future: Redis (caching) via redis-rs
- Future: Message queues (async processing) via lapin or rdkafka

### API Integration
- RESTful API endpoints with Axum
- OpenAPI documentation with utoipa at `/swagger`
- Health checks at `/health` and `/readiness`

### Tokio Runtime
- Multi-threaded runtime for production
- Async task spawning with `tokio::spawn`
- Timer operations with `tokio::time::sleep`
- Signal handling for graceful shutdown

## Scalability Considerations

### Horizontal Scaling
- Stateless application design
- JWT tokens (no session storage)
- Database connection pooling with sqlx
- Rate limiting per client with governor
- Thread-safe handlers with Send + Sync bounds

### Vertical Scaling
- Configurable database connections
- Efficient query generation with sqlx
- Connection pooling optimization
- Memory-efficient data structures
- Zero-cost abstractions from Rust

### Async Performance
- Tokio runtime for efficient async I/O
- Non-blocking database operations
- Concurrent request handling
- Efficient task scheduling

## Deployment Architecture

### Container Strategy
- Single container for application
- Docker Compose for local development
- Environment-specific configurations
- Health checks for orchestration
- Multi-stage Docker builds for Rust

### Configuration Management
- TOML or YAML configuration files per environment
- Environment variable overrides
- config crate for configuration loading
- Type-safe configuration with serde

### Rust-Specific Considerations
- Use `cargo-chef` for better Docker caching
- Optimize binary size with `lto = true` in release profile
- Strip symbols for smaller binaries
- Use musl target for static linking (optional)

## Technology Mapping

### Go → Rust Equivalents

| Go | Rust |
|----|------|
| Echo v4 | Axum 0.7+ |
| pgx/v5 | sqlx |
| SQLC | sqlx (compile-time checked) |
| golang-jwt/jwt/v5 | jsonwebtoken |
| Argon2id (crypto) | argon2 crate |
| Viper | config crate |
| Zerolog | tracing |
| validator/v10 | validator or garde |
| Swaggo | utoipa |
| testify | rstest |
| go.uber.org/mock | mockall |
| testcontainers | testcontainers-rs |

### Directory Structure Mapping

| Go | Rust |
|----|------|
| `cmd/server/main.go` | `src/main.rs` |
| `internal/app/app.go` | `src/app/mod.rs` |
| `internal/domain/user/entity/user.go` | `src/domain/user/entity.rs` |
| `internal/domain/user/repository/repository.go` | `src/domain/user/repository.rs` |
| `internal/domain/user/usecase/usecase.go` | `src/domain/user/service.rs` |
| `internal/domain/user/handler/handler.go` | `src/domain/user/handler.rs` |
| `internal/infrastructure/config/config.go` | `src/infrastructure/config/mod.rs` |
| `internal/infrastructure/db/postgres.go` | `src/infrastructure/db/postgres.rs` |
| `internal/infrastructure/logger/logger.go` | `src/infrastructure/logger/mod.rs` |
| `internal/infrastructure/password/passworder.go` | `src/infrastructure/password/mod.rs` |
| `pkg/middleware/` | `src/middleware/mod.rs` |
| `pkg/health/` | `src/health/mod.rs` |
| `sqlc/migrations/` | `migrations/` |
| `sqlc/queries/` | Inline with sqlx macros |

### Pattern Mapping

| Go Pattern | Rust Pattern |
|------------|--------------|
| Interfaces | Traits |
| Structs | Structs |
| Goroutines | Tokio tasks |
| Channels | tokio::sync channels |
| Context | tokio::sync Context or cancellation tokens |
| defer | Drop trait or explicit cleanup |
| panic/recover | Result<T, E> and ? operator |
| sync.Mutex | tokio::sync::Mutex or std::sync::Mutex |
| sync.RWMutex | tokio::sync::RwLock or std::sync::RwLock |
| waitGroup | tokio::task::JoinSet |
| select! | tokio::select! |
