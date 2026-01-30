# Context & Decisions: Zercle Rust Template

## Active Decisions

### 1. Clean Architecture Implementation
**Decision**: Implement Clean Architecture with explicit layer separation
**Rationale**: 
- Domain logic remains framework-independent
- Easier to test business logic in isolation
- Clear dependency direction (inward-pointing)
**Consequences**:
- More boilerplate for simple projects
- Requires discipline to maintain boundaries
- Better long-term maintainability

### 2. SQLx over ORM
**Decision**: Use SQLx for database operations instead of Diesel or SeaORM
**Rationale**:
- Compile-time checked SQL
- No hidden queries
- Better performance (no abstraction overhead)
- Easier to optimize queries
**Consequences**:
- Manual query writing required
- No automatic migration generation
- More verbose data mapping

### 3. Axum over Actix-web
**Decision**: Use Axum as web framework
**Rationale**:
- Native Tower middleware integration
- Better async/await ergonomics
- Strong typing throughout
- Maintained by Tokio team
**Consequences**:
- Smaller ecosystem than Actix
- Less middleware available
- Newer framework (less community knowledge)

### 4. YAML + Environment Configuration
**Decision**: Support both YAML files and environment variables
**Rationale**:
- YAML for local development (readable, versioned)
- Environment variables for deployment (12-factor app)
- Hierarchical override capability
**Consequences**:
- More complex configuration loading
- Potential confusion about precedence
- Requires careful documentation

### 5. Argon2id for Password Hashing
**Decision**: Use Argon2id with OWASP recommended parameters
**Rationale**:
- Winner of Password Hashing Competition
- Resistant to GPU attacks
- Memory-hard function
**Parameters**:
- Memory: 19456 KB
- Iterations: 2
- Parallelism: 1

## Dependencies

### Core Dependencies
| Dependency | Version | License | Purpose |
|------------|---------|---------|---------|
| axum | 0.8 | MIT | Web framework |
| tokio | 1 | MIT | Async runtime |
| sqlx | 0.8 | MIT/Apache | Database |
| serde | 1 | MIT/Apache | Serialization |
| argon2 | 0.5 | MIT/Apache | Password hashing |
| jsonwebtoken | 9 | MIT | JWT handling |

### Dev Dependencies
| Dependency | Version | Purpose |
|------------|---------|---------|
| tokio-test | 0.4 | Async testing |
| mockall | 0.14 | Mocking |
| rstest | 0.26 | Test parametrization |
| wiremock | 0.6.4 | HTTP mocking |

## Logic Summaries

### Configuration Loading
1. Check `APP_ENV` environment variable (default: `local`)
2. Attempt to load `configs/{env}.yaml`
3. If file exists, load and overlay environment variables
4. If file missing, load entirely from environment variables
5. Environment variables use `__` as separator (e.g., `DATABASE__HOST`)

### Database Connection
1. Parse connection string from configuration
2. Create `PgPoolOptions` with configured limits
3. Establish connection pool
4. Run pending migrations automatically
5. Pool available for dependency injection

### Migration Process
1. Migrations stored in `migrations/` directory
2. Naming: `YYYYMMDDHHMMSS_description.{up,down}.sql`
3. Applied in timestamp order
4. SQLx tracks applied migrations in `_sqlx_migrations` table
5. Both up and down migrations required

## Patterns in Use

### Repository Pattern (Planned)
```rust
#[async_trait]
trait UserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn create(&self, user: &User) -> Result<User>;
}
```

### Service Pattern (Planned)
```rust
struct UserService<R: UserRepository> {
    repository: R,
}

impl<R: UserRepository> UserService<R> {
    async fn register(&self, dto: RegisterDto) -> Result<User>;
    async fn authenticate(&self, dto: LoginDto) -> Result<AuthTokens>;
}
```

### Error Handling Pattern
```rust
#[derive(thiserror::Error, Debug)]
pub enum DomainError {
    #[error("User not found")]
    NotFound,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Validation failed: {0}")]
    Validation(String),
}

type Result<T> = std::result::Result<T, DomainError>;
```

## Open Questions

1. **API Versioning**: URL path (/v1/) vs Header (Accept-Version)?
2. **Pagination**: Offset-based vs Cursor-based for user lists?
3. **Rate Limiting**: In-memory vs Redis-based for distributed setup?
4. **Testing**: Test database per test or shared with transactions?

## Recent Changes Log

| Date | Change | Rationale |
|------|--------|-----------|
| 2026-01-30 | Initial project structure | Foundation setup |
| 2026-01-30 | Database migrations | Schema versioning |
| 2026-01-30 | Configuration system | Environment flexibility |

## Known Limitations

1. No implemented authentication endpoints yet (models only)
2. No request/response validation layer
3. No error handling standardization
4. CORS middleware configured but not integrated
5. No graceful shutdown implementation
6. Health check is basic (no DB connectivity check)
