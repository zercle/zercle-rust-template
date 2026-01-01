# Context & Decisions

## Architectural Decisions

### Clean Architecture Choice
**Decision:** Adopted Clean Architecture with DDD principles
**Rationale:** Separates business logic from infrastructure, improves testability, enables independent evolution of layers
**Impact:** All new domains must follow the established layer structure

### sqlx for Database Access
**Decision:** Use sqlx instead of raw SQL or ORM
**Rationale:** Type-safe queries with compile-time checking, better performance than ORMs, explicit SQL control, async/await support
**Impact:** All database queries must use sqlx macros (`sqlx::query!`, `sqlx::query_as!`)

### JWT Stateless Authentication
**Decision:** JWT tokens without server-side session storage
**Rationale:** Stateless design enables horizontal scaling, simpler architecture, no session management overhead
**Impact:** All protected routes require JWT middleware, tokens stored client-side

### Argon2id for Password Hashing
**Decision:** Argon2id algorithm for password hashing
**Rationale:** Memory-hard, resistant to GPU/ASIC attacks, recommended by security experts
**Impact:** All password operations must use the argon2 crate

### Axum Framework
**Decision:** Axum 0.7+ as HTTP framework
**Rationale:** High performance, minimal boilerplate, excellent Tower middleware support, async/await with Tokio, active community
**Impact:** All HTTP handlers use Axum patterns and Tower middleware

### Tokio Runtime
**Decision:** Tokio as async runtime
**Rationale:** Industry-standard async runtime, excellent performance, extensive ecosystem, battle-tested
**Impact:** All async operations run on Tokio runtime

## Domain Rules

### User Domain
**Business Rules:**
- Email must be unique across all users
- Password must be hashed before storage
- Users can only update their own profiles
- Email cannot be changed after registration
- Minimum full name length: 2 characters

**Validation Rules:**
- Email format validated by validator crate
- Password strength enforced by argon2 parameters
- Phone number is optional
- Full name required for registration

**Ownership Rules:**
- Users can only access their own profile
- Admin endpoints (if added) can access all users
- User ID extracted from JWT token for authorization

### Task Domain
**Business Rules:**
- Tasks must have an owner (user_id)
- Users can only access their own tasks
- Task status must be one of: pending, in_progress, completed, cancelled
- Task priority must be one of: low, medium, high, urgent
- Completed tasks automatically set completed_at timestamp

**Validation Rules:**
- Title is required
- Description is optional
- Due date is optional
- Status defaults to "pending"
- Priority defaults to "medium" if not specified

**Ownership Rules:**
- All task operations verify user ownership
- Cannot access/modify tasks owned by other users
- Task list filtered by user_id

## File & Component Summaries

### Core Application Files

**src/main.rs**
- Application entry point
- Loads environment-specific configuration
- Initializes Tokio runtime
- Initializes tracing logger
- Creates application state
- Starts Axum server
- Handles graceful shutdown

**src/app/mod.rs** or **src/lib.rs**
- Main application module
- Application state definition
- Dependency injection with Arc and traits
- Middleware setup (RequestID, Logger, Recovery, CORS, RateLimit)
- Route registration with Axum Router
- Server lifecycle management

**src/app/state.rs**
- Application state struct
- Shared resources (database pool, config, etc.)
- Implements Clone for Axum state

### Configuration

**src/infrastructure/config/mod.rs**
- Configuration structs for all components
- config crate-based configuration loading
- Environment variable support
- Type-safe configuration access with serde

**configs/*.toml** or **configs/*.yaml**
- Environment-specific configurations
- local, dev, uat, prod environments
- Database, JWT, logging, CORS, rate limit settings

### Database Layer

**src/infrastructure/db/postgres.rs**
- PostgreSQL database implementation
- Connection pooling configuration with sqlx PgPool
- Health check implementation
- sqlx query integration

**src/infrastructure/db/mod.rs**
- Database factory for creating connections
- Abstracts database type selection
- Currently supports PostgreSQL only
- Connection pool management

### Domain: User

**src/domain/user/entity.rs**
- User entity definition
- UUID-based primary key
- Fields: id, email, password, full_name, phone, timestamps
- Business logic methods

**src/domain/user/repository.rs**
- sqlx-based repository implementation
- Async CRUD operations for users
- Email uniqueness check
- Pagination support
- Repository trait definition

**src/domain/user/service.rs**
- Business logic for user operations
- Register, Login, GetProfile, UpdateProfile, DeleteAccount, ListUsers
- Password hashing and verification with argon2
- JWT token generation with jsonwebtoken
- Domain-specific error definitions with thiserror

**src/domain/user/handler.rs**
- HTTP handlers for user endpoints
- Request/response DTO mapping with serde
- Error handling and HTTP status codes
- Route registration
- Axum handler functions

**src/domain/user/request.rs**
- Request DTOs for user operations
- Validation with validator or garde derive macros
- Serde serialization

**src/domain/user/response.rs**
- Response DTOs for user operations
- Serde serialization

### Domain: Task

**src/domain/task/entity.rs**
- Task entity definition
- UUID-based primary key
- Fields: id, user_id, title, description, status, priority, due_date, completed_at, timestamps
- Business logic methods

**src/domain/task/repository.rs**
- sqlx-based repository implementation
- Async CRUD operations for tasks
- User filtering for list operations
- Ownership verification
- Repository trait definition

**src/domain/task/service.rs**
- Business logic for task operations
- CreateTask, GetTask, ListTasks, UpdateTask, DeleteTask
- Status and priority validation
- Ownership enforcement
- Domain-specific error definitions with thiserror

**src/domain/task/handler.rs**
- HTTP handlers for task endpoints
- Request/response DTO mapping with serde
- Error handling and HTTP status codes
- Protected routes only
- Axum handler functions

**src/domain/task/request.rs**
- Request DTOs for task operations
- Validation with validator or garde derive macros
- Serde serialization

**src/domain/task/response.rs**
- Response DTOs for task operations
- Serde serialization

### Infrastructure Components

**src/infrastructure/logger/mod.rs**
- tracing-based structured logger
- Configurable log levels and format
- Request ID integration
- Context-aware logging
- tracing-subscriber setup

**src/infrastructure/password/mod.rs**
- argon2 password hashing wrapper
- Configurable parameters
- Hash and verify operations
- Thread-safe implementation

**src/infrastructure/http/client/mod.rs**
- reqwest HTTP client wrapper
- For making external HTTP requests
- Configurable timeouts and retries
- Async client

**src/middleware/mod.rs**
- Custom middleware implementations
- JWT authentication (Tower layer)
- Request ID generation
- Structured logging
- CORS handling (tower-http)
- Rate limiting (governor or tower-governor)

**src/health/mod.rs**
- Health check handler
- Database connectivity check
- Readiness probe
- Axum handler for health endpoints

## Dependency Mapping

### Domain Dependencies
- **User Domain:** Depends on config, tracing, argon2, middleware (JWT), jsonwebtoken
- **Task Domain:** Depends on tracing, sqlx

### Infrastructure Dependencies
- **Database:** sqlx with PostgreSQL driver
- **Config:** config crate, serde
- **Logging:** tracing, tracing-subscriber
- **Validation:** validator or garde
- **Auth:** jsonwebtoken
- **Password:** argon2

### External Dependencies
- **PostgreSQL:** Primary database
- **Testcontainers-rs:** Integration testing
- **utoipa:** API documentation
- **Tokio:** Async runtime

## Key Implementation Details

### JWT Token Structure
- Contains user ID and email in claims
- Configurable expiration time
- Secret key from configuration
- Bearer token format in Authorization header
- Implemented with jsonwebtoken crate

### Database Connection Pool
- Min connections: 5
- Max connections: 25
- Connection lifetime: 1 hour
- Idle timeout: 10 minutes
- Health check period: 1 minute
- Implemented with sqlx PgPool

### Rate Limiting
- Configurable requests per time window
- Default: 100 requests per 60 seconds
- Applied at middleware level with Tower layer
- Per-client tracking
- Implemented with governor or tower-governor

### CORS Configuration
- Allowed origins configurable per environment
- Local: localhost:3000, localhost:8080
- Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS
- Headers: Authorization, Content-Type, X-Request-ID
- Implemented with tower-http CORS middleware

### Error Handling Pattern
```rust
// Service layer: Domain errors with thiserror
#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found")]
    NotFound,
    #[error("Email already exists")]
    EmailExists,
    #[error("Invalid credentials")]
    InvalidCredentials,
}

// Repository layer: Wrap with context
pub async fn get_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE email = $1", email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get user by email: {}", e);
            e
        })
}

// Handler layer: Map to HTTP status
impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            UserError::NotFound => (StatusCode::NOT_FOUND, "User not found"),
            UserError::EmailExists => (StatusCode::CONFLICT, "Email already exists"),
            UserError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

### Request Validation
- Use validator or garde crate derive macros
- Validate before business logic
- Return validation errors with field details
- Example: `#[validate(email)]`

### Pagination Pattern
```rust
// Standard pagination parameters
#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

// Repository returns data + total count
pub async fn list(
    &self,
    limit: u64,
    offset: u64,
) -> Result<(Vec<User>, i64), sqlx::Error> {
    let users = sqlx::query_as!(
        User,
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        limit as i64,
        offset as i64
    )
    .fetch_all(&self.pool)
    .await?;

    let total = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(&self.pool)
        .await?;

    Ok((users, total))
}

// Response includes pagination metadata
pub struct ListResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: u64,
    pub offset: u64,
}
```

## Testing Strategy

### Unit Tests
- Test service business logic
- Mock repository dependencies with mockall
- Test error paths and edge cases
- Located in same module as implementation (`#[cfg(test)]`)

### Integration Tests
- Test API endpoints end-to-end
- Use testcontainers for real database
- Test authentication flow
- Located in `tests/` directory

### Mock Generation
- Use mockall crate
- Generate mocks from traits
- Located in domain/*/mock.rs files
- Regenerate when traits change

### Test Helpers
- `tests/common/mod.rs` - Common test utilities
- `tests/integration/mod.rs` - Integration test setup
- Common test fixtures and utilities

### Async Testing
```rust
#[tokio::test]
async fn test_login_valid_credentials() {
    // Arrange
    let mock_repo = MockUserRepository::new();
    let use_case = UserService::new(mock_repo, config);

    // Act
    let result = use_case.login(request).await;

    // Assert
    assert!(result.is_ok());
}
```

## Migration Strategy

### Database Migrations
- sqlx-cli migration format
- Up and down migrations required
- Version naming: YYYYMMDD_NNN_description
- Apply migrations in order
- Rollback support with down migrations

### Schema Changes
- Add new migrations for schema changes
- Never modify existing migrations
- Use sqlx macros for queries after schema changes
- Test migrations in all environments

### Migration Commands
```bash
# Create migration
sqlx migrate add -r add_orders_table

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Prepare for offline mode
sqlx migrate info
```

## Configuration Management

### Environment Hierarchy
1. Base config from TOML/YAML file
2. Environment variable overrides
3. Default values in struct tags

### Configuration Files
- `configs/local.toml` - Local development
- `configs/dev.toml` - Development environment
- `configs/uat.toml` - User acceptance testing
- `configs/prod.toml` - Production

### Environment Variables
- `APP_ENV` - Environment selector (default: local)
- Database credentials via env vars in production
- JWT secret via env vars in production
- Never commit secrets to repository

### Config Loading
```rust
use config::{Config, Environment, File};

let config = Config::builder()
    .add_source(File::with_name("configs/local"))
    .add_source(Environment::with_prefix("APP"))
    .build()?;
```

## Deployment Considerations

### Docker Deployment
- Multi-stage build for optimization with cargo-chef
- Alpine-based final image
- Non-root user for security
- Health checks configured
- Port 3000 exposed
- Optimized binary with release profile

### Database Requirements
- PostgreSQL 12+ required
- Connection pool configuration important
- Migrations must be applied before startup
- Health check verifies connectivity

### Monitoring Points
- Health check endpoints
- Request/response logging with tracing
- Error logging with context
- Performance metrics (future)
- Database query performance (future)

### Rust-Specific Deployment
- Use `cargo build --release` for production builds
- Enable LTO in Cargo.toml for optimization
- Strip symbols for smaller binaries
- Use `cargo-chef` for better Docker caching
- Consider static linking with musl target

## Known Constraints

### Current Limitations
- Only PostgreSQL supported (no MySQL, SQLite)
- No caching layer implemented
- No message queue integration
- No distributed tracing
- No metrics collection
- Single-region deployment only

### Technical Debt
- Consider standardizing on sqlx for all domains
- Review async patterns for consistency
- Evaluate error handling strategy across domains

### Future Considerations
- Add Redis caching layer with redis-rs
- Implement message queue for async operations (lapin, rdkafka)
- Add Prometheus metrics with prometheus-client
- Implement distributed tracing with OpenTelemetry
- Add GraphQL support as alternative to REST (juniper, async-graphql)
- Consider gRPC for internal service communication (tonic)
- Add API versioning strategy

## Rust-Specific Considerations

### Ownership and Borrowing
- Use `&self` for read-only operations
- Use `&mut self` for write operations
- Use `Arc<T>` for shared state across async tasks
- Use `Clone` for cheap-to-clone types
- Use `Cow<T>` for borrowed or owned data

### Async/Await
- All I/O operations must be async
- Use `tokio::spawn` for concurrent tasks
- Use `tokio::join!` for concurrent operations
- Use `tokio::select!` for multiple futures
- Handle cancellation with `tokio::sync::CancellationToken`

### Error Handling
- Use `Result<T, E>` for fallible operations
- Use `?` operator for error propagation
- Use `thiserror` for custom error types
- Use `anyhow` for application-level errors
- Never use `unwrap()` or `expect()` in production code

### Thread Safety
- Use `Send + Sync` bounds for shared state
- Use `tokio::sync::Mutex` for async mutexes
- Use `std::sync::RwLock` for read-write locks
- Use `Arc<T>` for shared ownership across threads

### Performance
- Zero-cost abstractions from Rust
- Compile-time optimizations
- Memory safety without garbage collection
- Efficient async I/O with Tokio
- No runtime overhead for abstractions
