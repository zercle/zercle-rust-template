# Technical Standards & Guidelines

## Language & Runtime
- **Rust Version:** 1.80+
- **Edition:** 2021
- **Package:** zercle-rust-template

## Core Dependencies

### Web Framework
- **Axum 0.7+** - HTTP server framework built on Tokio and Tower
- **Tower middleware** - Request ID, logger, recovery, CORS, compression
- **Tokio** - Async runtime

### Database
- **sqlx** - PostgreSQL driver with compile-time checked queries
- **sqlx-core** - Core database functionality
- **sqlx-postgres** - PostgreSQL-specific implementation
- **deadpool** or **sqlx pool** - Connection pooling
- **testcontainers-rs** - Integration testing with real databases

### Authentication
- **jsonwebtoken** - JWT token generation and validation
- **argon2** - Password hashing (Argon2id algorithm)

### Configuration
- **config** - Configuration management
- **serde** - Serialization/deserialization
- **serde_yaml** or **serde_toml** - Configuration file format

### Logging
- **tracing** - Structured, async-aware logging
- **tracing-subscriber** - Logging subscriber implementation
- **tracing-appender** - Log file rotation

### Validation
- **validator** or **garde** - Request validation
- **serde** - Request/response serialization

### Documentation
- **utoipa** - OpenAPI/Swagger documentation generation
- **utoipa-swagger-ui** - Swagger UI integration

### Testing
- **rstest** - Table-driven testing
- **mockall** - Mock generation
- **testcontainers-rs** - Integration testing
- **proptest** - Property-based testing (optional)

## Coding Standards

### Naming Conventions
- **Files:** snake_case (e.g., `user_handler.rs`)
- **Modules:** snake_case (e.g., `handler`, `usecase`, `repository`)
- **Traits:** PascalCase describing capability (e.g., `UserRepository`)
- **Structs:** PascalCase (e.g., `UserUseCase`, `UserHandler`)
- **Enums:** PascalCase with variants in PascalCase
- **Constants:** UPPER_SNAKE_CASE
- **Functions:** snake_case
- **Variables:** snake_case

### Code Organization
- **Module structure:** One responsibility per module
- **File size:** Keep files focused and under 300 lines when possible
- **Function length:** Prefer functions under 50 lines
- **Public items:** Must have rustdoc comments (`///`)
- **Error handling:** Always handle errors with `Result<T, E>`, never use `unwrap()` in production

### Rust Idioms
- Use `&str` for string views, `String` for owned strings
- Prefer `Option<T>` over nullable references
- Use `Result<T, E>` for fallible operations
- Leverage `?` operator for error propagation
- Use `match` for exhaustive pattern matching
- Prefer iterators over loops when appropriate
- Use `async fn` for async operations
- Use `#[tokio::test]` for async tests

### Design Patterns

**Repository Pattern:**
- Abstract data access behind traits
- Domain entities mapped to database models
- Repository implementations in infrastructure layer
- Use async traits for database operations

**Service Pattern (Use Case):**
- Business logic encapsulated in services
- Coordinate between repositories and handlers
- Domain-specific error definitions with `thiserror`

**Factory Pattern:**
- Database factory for creating connections
- Configuration-based instantiation
- Use `Arc` for shared state

**Middleware Pattern:**
- Request/response processing pipeline with Tower middleware
- Cross-cutting concerns (auth, logging, CORS)
- Use Tower layers for composition

### SOLID Principles

**Single Responsibility:**
- Each module has one clear purpose
- Functions do one thing well
- Structs/traits focused on single capability

**Open/Closed:**
- Traits for extensibility
- New features through new implementations
- Avoid modifying existing, stable code

**Liskov Substitution:**
- Trait contracts honored by implementations
- Mock implementations behave like real ones

**Interface Segregation:**
- Small, focused traits
- Clients depend only on needed methods

**Dependency Inversion:**
- Depend on abstractions (traits)
- High-level modules don't depend on low-level
- Inversion of Control through dependency injection

## Testing Guidelines

### Test Structure
- **Unit tests:** Test individual functions/methods with `#[test]`
- **Integration tests:** Test component interactions in `tests/` directory
- **Async tests:** Use `#[tokio::test]` for async operations
- **Mock tests:** Use mockall for dependency mocking

### Test Organization
```
src/
  domain/
    user/
      handler.rs
      handler_test.rs
      usecase.rs
      usecase_test.rs
tests/
  integration/
    api_test.rs
```

### Testing Best Practices
- Write tests for critical business logic
- Aim for >80% coverage on core paths
- Use table-driven tests with rstest for multiple scenarios
- Mock external dependencies (database, HTTP clients)
- Use testcontainers for real database integration tests
- Test error paths, not just happy paths
- Use `assert_eq!`, `assert_ne!`, `assert!` macros
- Use `anyhow` or `eyre` for test error handling

### Test Naming
- `test_<function_name>_<scenario>_<expected_result>`
- Example: `test_login_valid_credentials_returns_token`

### Async Testing
```rust
#[tokio::test]
async fn test_login_valid_credentials() {
    // Arrange
    let mock_repo = MockUserRepository::new();
    let use_case = UserUseCase::new(mock_repo, config);

    // Act
    let result = use_case.login(request).await;

    // Assert
    assert!(result.is_ok());
}
```

## Security Standards

### Password Storage
- Always use Argon2id for password hashing
- Configurable memory, iterations, parallelism
- Never store plaintext passwords
- Use `argon2` crate with secure defaults

### Authentication
- JWT tokens for stateless authentication
- Token expiration configurable
- Secret key must be environment-specific
- Validate tokens on protected routes
- Use `jsonwebtoken` crate with HS256 or RS256

### Input Validation
- Validate all user inputs
- Use validator or garde crate with derive macros
- Sanitize database queries (sqlx prevents SQL injection)
- Validate file uploads (size, type)
- Use `serde` for request/response validation

### CORS Configuration
- Whitelist allowed origins per environment
- Configure allowed methods and headers
- Use tower-http CORS middleware
- Use secure defaults for production

### Rate Limiting
- Configurable requests per time window
- Apply to API endpoints with governor or tower-governor
- Prevent abuse and DoS attacks

### Memory Safety
- Rust's ownership system prevents buffer overflows
- No null pointer dereferences
- No data races in safe Rust
- Compile-time memory safety guarantees

## Database Standards

### Migrations
- Use sqlx-cli or sea-orm migrations
- Up and down migrations required
- Version with timestamp format: `YYYYMMDD_NNN_description`
- Place in `migrations/` directory
- Use `sqlx migrate run` to apply migrations

### Queries
- Use sqlx for type-safe queries
- SQL queries inline with `sqlx::query!()` macro for compile-time checking
- Or use `sqlx::query_as!()` for automatic mapping
- Parameterized queries (sqlx handles this)
- Use transactions with `.begin()` and `.commit()`

### Connection Pooling
- Configure min/max connections
- Set connection lifetime and idle timeout
- Use deadpool or sqlx built-in pool
- Adjust based on application load
- Use `PgPoolOptions` for configuration

### Async Database Operations
- All database operations must be async
- Use `sqlx::PgPool` for connection pooling
- Use `&mut PgConnection` for transactions
- Handle `sqlx::Error` appropriately

## Error Handling

### Error Types
- **Domain errors:** Business rule violations (e.g., `UserNotFound`)
- **Repository errors:** Data access failures
- **Infrastructure errors:** External service failures
- **Validation errors:** Input validation failures

### Error Handling Pattern
- Use `thiserror` for custom error types
- Use `anyhow` for application-level error handling
- Use `?` operator for error propagation
- Implement `std::error::Error` trait
- Implement `Display` trait for error messages

### Error Definition Example
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found")]
    NotFound,
    #[error("Email already exists")]
    EmailExists,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

### HTTP Status Codes
- 200 OK - Successful GET/PUT/PATCH
- 201 Created - Successful POST
- 400 Bad Request - Validation errors
- 401 Unauthorized - Missing/invalid JWT
- 404 Not Found - Resource not found
- 409 Conflict - Duplicate resources
- 500 Internal Server Error - Unexpected errors

### Error Response Format
```rust
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}
```

## Logging Standards

### Log Levels
- **TRACE:** Very detailed diagnostic information
- **DEBUG:** Detailed diagnostic information
- **INFO:** General informational messages
- **WARN:** Warning messages for potential issues
- **ERROR:** Error events that might still allow continued operation

### Log Format
- Structured JSON logging with tracing
- Include request ID for tracing
- Contextual fields (user_id, action, resource)
- Timestamps in ISO 8601 format
- Use `tracing::instrument!` for function tracing

### What to Log
- Application startup/shutdown
- Request/response for API calls (with request ID)
- Errors with stack traces
- Business events (user registration, task creation)
- Performance metrics (slow queries, long-running operations)

### Tracing Setup
```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
    ))
    .with(tracing_subscriber::fmt::layer())
    .init();
```

## API Standards

### RESTful Design
- Use appropriate HTTP methods (GET, POST, PUT, PATCH, DELETE)
- Resource-based URLs (e.g., `/api/v1/users/:id`)
- Query parameters for filtering and pagination
- Consistent response format

### Response Format
```json
{
  "data": { ... },
  "error": null,
  "meta": { "total": 100, "page": 1 }
}
```

### Versioning
- URL-based versioning: `/api/v1/`
- Backward compatibility within major versions
- Deprecation notices for breaking changes

### Documentation
- OpenAPI/Swagger documentation with utoipa
- Auto-generated from code annotations
- Example requests/responses
- Authentication requirements documented

### Axum Handler Pattern
```rust
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    // Handler logic
}
```

## Deployment Guidelines

### Docker
- Multi-stage builds for optimization
- Alpine-based images for smaller size
- Non-root user for security
- Health checks defined in Dockerfile
- Use `cargo-chef` for better caching

### Configuration
- Environment-specific configs (local, dev, uat, prod)
- Sensitive data via environment variables
- Never commit secrets to repository
- Use `.env` files for local development

### Health Checks
- `/health` - Application health
- `/readiness` - Readiness for traffic
- Database connectivity check
- Dependency service checks

## Performance Guidelines

### Database
- Use connection pooling with deadpool or sqlx pool
- Optimize queries with proper indexes
- Batch operations when possible
- Use prepared statements (sqlx handles this)
- Use async operations for database I/O

### HTTP
- Enable compression with tower-http compression middleware
- Use appropriate cache headers
- Implement rate limiting
- Monitor response times
- Use async handlers

### Memory
- Reuse objects where possible
- Avoid allocations in hot paths
- Use value types for small structs
- Profile before optimizing
- Use `Box` for large data structures
- Use `Arc` for shared state

### Async Runtime
- Use Tokio runtime with appropriate thread pool size
- Configure Tokio for multi-threaded or single-threaded execution
- Use `tokio::spawn` for concurrent tasks
- Use `tokio::join!` for concurrent operations

### Zero-Cost Abstractions
- Leverage Rust's zero-cost abstractions
- Use iterators and functional patterns
- Avoid unnecessary allocations
- Use `Cow` for borrowed or owned data

## Code Quality

### Linting
- Use `cargo clippy` for linting
- Configure in `.clippy.toml`
- Run in CI/CD pipeline
- Fix all clippy warnings

### Formatting
- Use `cargo fmt` for code formatting
- Use `rustfmt.toml` for configuration
- Run in CI/CD pipeline
- Enforce consistent formatting

### Code Review Checklist
- Follows coding standards
- Tests included and passing
- Error handling complete
- Documentation updated
- No security vulnerabilities
- Performance considered
- No `unwrap()` or `expect()` in production code
- Proper use of `Result<T, E>` and `Option<T>`

### Documentation
- Rustdoc comments for public items (`///`)
- README with setup instructions
- API documentation (OpenAPI/Swagger)
- Architecture documentation (Memory Bank)
- Example code in documentation

### Cargo Commands
```bash
# Build
cargo build

# Build release
cargo build --release

# Run
cargo run

# Test
cargo test

# Test with output
cargo test -- --nocapture

# Check
cargo check

# Clippy
cargo clippy

# Format
cargo fmt

# Doc
cargo doc --open

# Update dependencies
cargo update

# Add dependency
cargo add <crate>

# Remove dependency
cargo remove <crate>
```

### Coverage
- Use `cargo-tarpaulin` or `cargo-llvm-cov` for coverage
- Aim for >80% coverage on critical paths
- Generate coverage reports
