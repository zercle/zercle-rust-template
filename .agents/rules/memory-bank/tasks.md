# Operational Workflows

## Test-Driven Development (TDD)

### TDD Cycle
1. **Red:** Write a failing test for the desired behavior
2. **Green:** Write minimal code to make the test pass
3. **Refactor:** Improve code while keeping tests green

### When to Write Tests
- **Before implementing:** New features or business logic
- **Before fixing bugs:** Reproduce the bug with a test
- **After refactoring:** Ensure behavior unchanged
- **Critical paths:** Authentication, authorization, data persistence

### Test Organization

**Unit Tests:**
- Location: Same module as implementation (`#[cfg(test)]` module)
- Scope: Single function or method
- Dependencies: Mock external dependencies
- Examples: `src/domain/user/service.rs` (test module)

**Integration Tests:**
- Location: `tests/` directory
- Scope: End-to-end API flows
- Dependencies: Real database (testcontainers-rs)
- Examples: `tests/integration/api_test.rs`

**Mock Tests:**
- Location: `tests/mock/` or inline in test modules
- Scope: Database interactions
- Dependencies: Mock implementations
- Examples: `tests/mock/db_test.rs`

### Test Structure Template

```rust
#[tokio::test]
async fn test_login_valid_credentials_returns_token() {
    // Arrange
    let ctx = Context::new();
    let mock_repo = MockUserRepository::new();
    let use_case = UserService::new(mock_repo, config, argon2_config, log);

    // Setup expectations
    mock_repo
        .expect_get_by_email()
        .returning(|_| Ok(None));

    // Act
    let result = use_case.login(ctx, request).await;

    // Assert
    assert!(result.is_ok());
    assert!(result.unwrap().token.is_some());
}
```

### Table-Driven Tests

```rust
#[rstest]
#[case("user@example.com", false)]
#[case("invalid", true)]
#[case("", true)]
fn test_validate_email(#[case] email: &str, #[case] want_err: bool) {
    let result = validate_email(email);
    assert_eq!(result.is_err(), want_err);
}
```

### Running Tests

**All tests:**
```bash
cargo test
```

**Specific package:**
```bash
cargo test --package zercle_rust_template
```

**Specific module:**
```bash
cargo test --lib domain::user::service
```

**With coverage:**
```bash
cargo tarpaulin --out Html
```

**Coverage report:**
```bash
cargo llvm-cov --html
```

**Integration tests:**
```bash
cargo test --test integration
```

**Single test:**
```bash
cargo test test_login_valid_credentials
```

**Test with output:**
```bash
cargo test -- --nocapture
```

### Test Coverage Goals
- **Critical business logic:** >90%
- **Domain services:** >80%
- **Handlers:** >70%
- **Infrastructure:** >60%
- **Overall:** >70%

## Refactoring Procedures

### When to Refactor
- Code duplication detected
- Complex functions (>50 lines)
- God objects with too many responsibilities
- Poor naming or unclear intent
- Performance bottlenecks identified
- Adding new features becomes difficult
- Clippy warnings

### Refactoring Checklist
- [ ] Ensure tests exist and pass
- [ ] Identify the smell/problem
- [ ] Plan the refactoring approach
- [ ] Make small, incremental changes
- [ ] Run tests after each change
- [ ] Verify behavior unchanged
- [ ] Run `cargo clippy` and fix warnings
- [ ] Update documentation if needed
- [ ] Commit with clear message

### Common Refactorings

**Extract Method:**
- Move code to a new function
- Give it a descriptive name
- Replace original code with function call

**Extract Trait:**
- Identify common behavior
- Create trait with methods
- Implement trait in concrete types
- Update dependencies to use trait

**Replace Magic Numbers:**
- Identify constants in code
- Create named constants
- Replace numbers with constants
- Add documentation

**Simplify Conditional:**
- Use guard clauses
- Replace nested if-else with match
- Extract complex conditions to named functions

**Remove Dead Code:**
- Identify unused code
- Remove or comment out
- Run tests to verify
- Commit removal

### Refactoring Example

**Before:**
```rust
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterUser>,
) -> Result<Json<LoginResponse>, ApiError> {
    if req.email.is_empty() {
        return Err(ApiError::Validation("Email is required".to_string()));
    }
    if req.full_name.is_empty() {
        return Err(ApiError::Validation("Full name is required".to_string()));
    }
    if req.password.is_empty() {
        return Err(ApiError::Validation("Password is required".to_string()));
    }
    // ... more code
}
```

**After:**
```rust
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterUser>,
) -> Result<Json<LoginResponse>, ApiError> {
    validate_register_request(&req)?;

    // ... more code
}

fn validate_register_request(req: &RegisterUser) -> Result<(), ApiError> {
    if req.email.is_empty() {
        return Err(ApiError::Validation("Email is required".to_string()));
    }
    if req.full_name.is_empty() {
        return Err(ApiError::Validation("Full name is required".to_string()));
    }
    if req.password.is_empty() {
        return Err(ApiError::Validation("Password is required".to_string()));
    }
    Ok(())
}
```

## Code Review Checklist

### General Review
- [ ] Code follows project coding standards
- [ ] Naming is clear and descriptive (snake_case for functions, PascalCase for types)
- [ ] Functions are small and focused
- [ ] No code duplication
- [ ] Comments explain "why", not "what"
- [ ] No commented-out code left behind
- [ ] Proper error handling throughout (Result<T, E>)
- [ ] Logging at appropriate levels with tracing
- [ ] No `unwrap()` or `expect()` in production code
- [ ] Clippy warnings addressed

### Architecture Review
- [ ] Follows clean architecture principles
- [ ] Dependencies point inward
- [ ] Domain logic isolated from infrastructure
- [ ] Traits used for external dependencies
- [ ] No circular dependencies
- [ ] Proper separation of concerns
- [ ] Async/await used appropriately
- [ ] Ownership and borrowing handled correctly

### Security Review
- [ ] Input validation on all user inputs
- [ ] SQL injection prevention (sqlx handles this)
- [ ] Authentication/authorization enforced
- [ ] Sensitive data not logged
- [ ] Secrets not hardcoded
- [ ] CORS properly configured
- [ ] Rate limiting applied
- [ ] Passwords hashed with argon2

### Performance Review
- [ ] No N+1 query problems
- [ ] Database queries optimized
- [ ] Connection pooling configured
- [ ] No unnecessary allocations
- [ ] Efficient data structures used
- [ ] Caching considered where appropriate
- [ ] Async operations used for I/O

### Testing Review
- [ ] Tests added for new functionality
- [ ] Tests cover edge cases
- [ ] Tests are readable and maintainable
- [ ] Mocks used appropriately (mockall)
- [ ] Test coverage adequate
- [ ] Integration tests included for API changes
- [ ] Async tests use `#[tokio::test]`

### Documentation Review
- [ ] Rustdoc comments on public items (`///`)
- [ ] API documentation updated (OpenAPI/utoipa)
- [ ] README updated if needed
- [ ] Architecture docs updated if major change
- [ ] Migration files documented

### Specific Domain Reviews

**User Domain:**
- [ ] Password hashing with argon2
- [ ] Email uniqueness enforced
- [ ] JWT token properly generated
- [ ] User ownership verified

**Task Domain:**
- [ ] Task ownership verified
- [ ] Status values validated
- [ ] Priority values validated
- [ ] Due date handling correct

**Database:**
- [ ] Migration files created
- [ ] sqlx queries updated
- [ ] Indexes added if needed
- [ ] Foreign keys defined

## Debugging Protocols

### Debugging Workflow

1. **Reproduce the Issue**
   - Get exact steps to reproduce
   - Identify affected environment
   - Gather error messages and logs
   - Note request/response data

2. **Gather Information**
   - Check application logs with tracing
   - Review database state
   - Examine request/response
   - Check configuration values

3. **Formulate Hypothesis**
   - Based on symptoms
   - Consider recent changes
   - Review related code
   - Check known issues

4. **Test Hypothesis**
   - Add logging to verify
   - Write reproduction test
   - Use debugger if needed
   - Isolate the problem

5. **Implement Fix**
   - Write minimal fix
   - Add tests for fix
   - Verify fix works
   - Check for side effects

### Debugging Tools

**Logging:**
```rust
tracing::debug!(
    user_id = %user_id,
    task_id = %task_id,
    "Processing request"
);

tracing::error!(
    error = %err,
    task_id = %task_id,
    "Failed to update task"
);
```

**Structured Logging:**
- Include request ID in all logs
- Use consistent field names
- Log at appropriate levels
- Include context for errors
- Use `tracing::instrument!` for function tracing

**Error Inspection:**
```rust
if let Err(err) = result {
    tracing::error!(
        error = %err,
        operation = "create_user",
        email = %req.email,
        "Operation failed"
    );
    // Use error.kind() or error.downcast_ref() for error checking
}
```

**Database Debugging:**
```bash
# Connect to database
psql -h localhost -U postgres -d postgres

# Check recent queries
SELECT * FROM pg_stat_statements ORDER BY total_time DESC LIMIT 10;

# Check connection pool
SELECT * FROM pg_stat_activity;
```

**HTTP Debugging:**
```bash
# Check API endpoint
curl -X GET http://localhost:3000/health

# With authentication
curl -X GET http://localhost:3000/api/v1/users \
  -H "Authorization: Bearer <token>"

# Verbose output
curl -v http://localhost:3000/api/v1/tasks
```

### Common Issues & Solutions

**Database Connection Issues:**
- Check database is running
- Verify connection string
- Check connection pool settings
- Review firewall rules
- Check sqlx connection configuration

**Authentication Failures:**
- Verify JWT secret matches
- Check token expiration
- Validate token format
- Review middleware configuration
- Check jsonwebtoken claims

**Performance Issues:**
- Check database query performance
- Review connection pool settings
- Profile with tokio-console or flamegraph
- Check for N+1 queries
- Review async task spawning

**Test Failures:**
- Run tests with verbose output (`-- --nocapture`)
- Check test data setup
- Verify mock expectations
- Review test isolation
- Check async test setup

**Compilation Errors:**
- Check ownership and borrowing
- Verify trait bounds
- Review async function signatures
- Check lifetime annotations
- Use `cargo check` for early errors

### Adding Debug Logging

**Before Production:**
```rust
#[tracing::instrument(skip(self))]
pub async fn login(
    &self,
    ctx: Context,
    req: LoginUser,
) -> Result<LoginResponse, UserError> {
    tracing::debug!(email = %req.email, "Login attempt");

    let user_model = self.repo.get_by_email(&ctx, &req.email).await?;
    if user_model.is_none() {
        tracing::error!(email = %req.email, "User not found");
        return Err(UserError::InvalidCredentials);
    }

    // ... rest of code
}
```

**Remove Before Production:**
- Remove debug-level logs
- Keep error and warn logs
- Ensure no sensitive data in logs
- Set appropriate log level in production

### Performance Debugging

**Enable Tokio Console:**
```rust
use tokio_console::console_layer;

#[tokio::main]
async fn main() {
    console_layer().init();
    // ... rest of code
}
```

**Profile with Flamegraph:**
```bash
cargo install flamegraph
cargo flamegraph --bin zercle-rust-template
```

**Profile Memory:**
```bash
cargo install heaptrack
heaptrack ./target/release/zercle-rust-template
```

### Integration Testing Debugging

**Run Single Test:**
```bash
cargo test --test integration test_login -- --nocapture
```

**Keep Database Running:**
```rust
// Testcontainers automatically cleans up
// Use --nocapture to see logs
```

**View Test Database:**
```bash
# Get container ID
docker ps

# Connect to test database
docker exec -it <container_id> psql -U postgres -d postgres
```

## Adding a New Domain

### Step-by-Step Process

1. **Create Domain Structure**
   ```
   src/domain/<domain>/
     entity.rs
     handler.rs
     repository.rs
     service.rs
     request.rs
     response.rs
     mock.rs
     mod.rs
   ```

2. **Define Entity**
   - Create entity in `entity.rs`
   - Add UUID primary key
   - Add timestamps (created_at, updated_at)
   - Add business logic methods
   - Implement serde Serialize/Deserialize

3. **Create Trait**
   - Define Repository, Service traits
   - Follow existing patterns
   - Use domain-specific types
   - Add async methods

4. **Implement Repository**
   - Create SQL queries with sqlx macros
   - Implement repository trait
   - Handle errors appropriately
   - Use async/await

5. **Implement Service**
   - Create business logic
   - Define domain-specific errors with thiserror
   - Implement validation rules
   - Add logging with tracing

6. **Implement Handler**
   - Create HTTP handlers
   - Map request/response DTOs
   - Handle errors
   - Register routes
   - Use Axum handler patterns

7. **Add Tests**
   - Unit tests for service
   - Integration tests for API
   - Mock tests for repository
   - Use `#[tokio::test]` for async tests

8. **Update Application**
   - Wire dependencies in `src/app/mod.rs`
   - Register routes
   - Update OpenAPI documentation with utoipa

9. **Update Documentation**
   - Add to architecture.md
   - Update context.md
   - Add API examples

## Database Migration Workflow

### Creating a Migration

1. **Create Migration File**
   ```bash
   # Format: YYYYMMDD_NNN_description
   sqlx migrate add -r add_orders_table
   ```

2. **Write Up Migration**
   ```sql
   -- migrations/20260101_003000000_add_orders_table.up.sql
   CREATE TABLE orders (
       id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
       user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
       total DECIMAL(10,2) NOT NULL,
       status VARCHAR(50) NOT NULL DEFAULT 'pending',
       created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
       updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
   );

   CREATE INDEX idx_orders_user_id ON orders(user_id);
   ```

3. **Write Down Migration**
   ```sql
   -- migrations/20260101_003000000_add_orders_table.down.sql
   DROP INDEX IF EXISTS idx_orders_user_id;
   DROP TABLE IF EXISTS orders;
   ```

4. **Apply Migration**
   ```bash
   sqlx migrate run
   ```

5. **Verify Migration**
   ```bash
   sqlx migrate info
   ```

### Migration Best Practices
- Always write both up and down migrations
- Use transactions for complex changes
- Add indexes for foreign keys
- Consider data migration for schema changes
- Test migrations on development first
- Never modify existing migrations

### sqlx Commands
```bash
# Add migration
sqlx migrate add -r description

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Show migration status
sqlx migrate info

# Prepare for offline mode
sqlx migrate info --database-url postgres://...
```

## Running the Application

### Development
```bash
# Set environment
export APP_ENV=local

# Run with hot reload (add cargo-watch)
cargo watch -x run

# Or standard run
cargo run
```

### Production
```bash
# Build
cargo build --release

# Run
./target/release/zercle-rust-template
```

### Docker
```bash
# Build image
docker build -t zercle-rust-template .

# Run container
docker run -p 3000:3000 \
  -e APP_ENV=prod \
  -e DATABASE_URL=... \
  zercle-rust-template
```

### Docker Compose
```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

## Common Commands

### Linting
```bash
# Run clippy
cargo clippy

# Fix issues
cargo clippy --fix

# All warnings as errors
cargo clippy -- -D warnings
```

### Formatting
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Dependencies
```bash
# Add dependency
cargo add <crate>

# Add dev dependency
cargo add --dev <crate>

# Update dependencies
cargo update

# Check for outdated
cargo outdated

# Remove dependency
cargo remove <crate>

# Check for security vulnerabilities
cargo audit
```

### Documentation
```bash
# Generate documentation
cargo doc --open

# Document private items
cargo doc --document-private-items
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests in parallel
cargo test -- --test-threads=4

# Run specific test
cargo test test_name

# Run tests with coverage
cargo tarpaulin --out Html
```

### Build
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Check without building
cargo check
```

## Environment Setup

### Prerequisites
- Rust 1.80+ (install via rustup)
- PostgreSQL 12+
- Docker (optional, for containerized deployment)

### Local Development
1. Clone repository
2. Copy `.env.example` to `.env`
3. Configure database connection
4. Run migrations
5. Start application

### Rust Installation
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update Rust
rustup update

# Add components
rustup component add clippy rustfmt
```

### Database Setup
```bash
# Start PostgreSQL with Docker
docker run --name postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=postgres \
  -p 5432:5432 \
  -d postgres:15

# Run migrations
sqlx migrate run
```

### Seed Data
```bash
# Run seed script
./scripts/seed-db.sh
```

### Development Tools
```bash
# Install useful tools
cargo install cargo-watch    # Hot reload
cargo install cargo-tarpaulin # Coverage
cargo install cargo-audit    # Security audit
cargo install cargo-outdated # Check outdated deps
cargo install sqlx-cli      # Database migrations
cargo install tokio-console  # Async debugging
```

## Rust-Specific Workflows

### Error Handling
- Always use `Result<T, E>` for fallible operations
- Use `?` operator for error propagation
- Use `thiserror` for custom error types
- Use `anyhow` for application-level errors
- Never use `unwrap()` or `expect()` in production code

### Async/Await
- All I/O operations must be async
- Use `#[tokio::test]` for async tests
- Use `tokio::spawn` for concurrent tasks
- Use `tokio::join!` for concurrent operations
- Handle cancellation appropriately

### Ownership and Borrowing
- Use `&self` for read-only operations
- Use `&mut self` for write operations
- Use `Arc<T>` for shared state across async tasks
- Use `Clone` for cheap-to-clone types
- Use `Cow<T>` for borrowed or owned data

### Testing
- Use `#[test]` for synchronous tests
- Use `#[tokio::test]` for async tests
- Use `rstest` for table-driven tests
- Use `mockall` for mocking
- Use `testcontainers-rs` for integration tests

### Performance
- Profile with `tokio-console` or `flamegraph`
- Use `cargo flamegraph` for flamegraphs
- Use `cargo tarpaulin` for coverage
- Use `cargo clippy` for linting
- Use `cargo audit` for security checks
