# Task Workflows: Zercle Rust Template

## Development Workflow

### Starting New Feature
1. Create feature branch from `develop`
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/description
   ```

2. Review memory bank for context
   - Check `architecture.md` for patterns
   - Check `tech.md` for standards
   - Check `context.md` for decisions

3. Implement with tests
   - Write failing test first (TDD preferred)
   - Implement minimal code to pass
   - Refactor while keeping tests green

4. Update documentation
   - Add rustdoc comments
   - Update memory bank if architectural changes

5. Create PR with checklist:
   - [ ] Tests pass
   - [ ] Clippy warnings resolved
   - [ ] Code formatted (`cargo fmt`)
   - [ ] Memory bank updated if needed
   - [ ] Breaking changes documented

### Running Locally
```bash
# Start PostgreSQL
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:15

# Run migrations and start server
APP_ENV=local cargo run

# Or use the script
./scripts/run-dev.sh
```

### Testing
```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --test unit

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## TDD Workflow

### 1. Red: Write Failing Test
```rust
#[tokio::test]
async fn should_create_user_with_valid_data() {
    // Arrange
    let service = create_test_service();
    let dto = RegisterDto {
        email: "test@example.com".to_string(),
        password: "Secure123!".to_string(),
    };
    
    // Act
    let result = service.register(dto).await;
    
    // Assert
    assert!(result.is_ok());
}
```

### 2. Green: Minimal Implementation
```rust
pub async fn register(&self, dto: RegisterDto) -> Result<User> {
    // Minimal implementation to pass test
    let user = User::new(/* ... */);
    self.repository.create(&user).await
}
```

### 3. Refactor: Improve Code
- Extract validation logic
- Optimize database queries
- Improve error messages

## Refactoring Workflow

### Before Refactoring
1. Ensure tests cover the code
2. Identify refactoring goal (performance, readability, etc.)
3. Review `architecture.md` for patterns

### During Refactoring
1. Make small, incremental changes
2. Run tests after each change
3. Use compiler errors as guide

### After Refactoring
1. Run full test suite
2. Run clippy for linting
3. Update documentation
4. Document significant changes in `context.md`

### Common Refactorings
| Refactoring | Trigger | Approach |
|-------------|---------|----------|
| Extract Function | Code duplication | Create shared function with generic params |
| Extract Trait | Multiple implementations | Define trait, implement for types |
| Move Module | Wrong layer | Move to correct layer, update visibility |
| Rename | Unclear naming | Use IDE rename, verify with compile |

## Code Review Workflow

### As Author
1. Self-review before requesting review
   - [ ] Logic is correct
   - [ ] Error handling is comprehensive
   - [ ] No unwrap/expect in production code
   - [ ] Tests cover edge cases
   - [ ] Documentation updated

2. Provide context in PR description
   - What changed and why
   - Link to related issues
   - Breaking changes noted

### As Reviewer
1. Check architectural alignment
   - Does it follow Clean Architecture?
   - Are dependencies pointing inward?
   - Is it in the correct layer?

2. Check code quality
   - Error handling completeness
   - Test coverage adequacy
   - Performance considerations
   - Security implications

3. Provide actionable feedback
   - Suggest specific improvements
   - Explain reasoning
   - Distinguish blocking vs. suggestions

## Debugging Workflow

### When Test Fails
1. Reproduce failure consistently
2. Add logging/tracing to understand flow
3. Isolate the failing component
4. Use `dbg!()` macro for quick inspection
5. Add `#[tracing::instrument]` for async debugging

### When Production Issue
1. Check logs for error context
2. Reproduce locally if possible
3. Add metrics/logging around suspected area
4. Deploy fix behind feature flag if risky

### Debugging Tools
| Tool | Use Case |
|------|----------|
| `dbg!()` | Quick value inspection |
| `tracing::info!()` | Structured logging |
| `cargo test -- --nocapture` | See println! in tests |
| IDE debugger | Step-through debugging |

## Migration Workflow

### Creating New Migration
```bash
# Create migration files
sqlx migrate add description

# Edit generated files
# - migrations/YYYYMMDDHHMMSS_description.up.sql
# - migrations/YYYYMMDDHHMMSS_description.down.sql
```

### Writing Migrations
- Always provide both up and down
- Make migrations idempotent (IF NOT EXISTS)
- Test on copy of production data
- Never modify existing migration after commit

### Running Migrations
```bash
# Auto-run on startup (configured)
cargo run

# Manual run
sqlx migrate run

# Revert
sqlx migrate revert
```

## Deployment Workflow

### Pre-deployment Checklist
- [ ] All tests pass
- [ ] Migrations tested
- [ ] Environment variables configured
- [ ] Logging level appropriate
- [ ] Health check endpoint responds

### Deployment Steps
1. Run database migrations
2. Deploy new version
3. Verify health check
4. Monitor error rates
5. Rollback if errors spike

### Rollback Procedure
1. Identify issue from logs
2. Revert to previous version
3. Revert database migrations if needed
4. Post-mortem and fix forward

## Documentation Update Workflow

### When to Update
- New architectural decision → `context.md`
- Technology change → `tech.md`
- New feature → `product.md`
- Pattern change → `architecture.md`

### Update Process
1. Update relevant memory bank file
2. Add timestamp to changes
3. Document rationale for significant changes
4. Verify consistency across files
