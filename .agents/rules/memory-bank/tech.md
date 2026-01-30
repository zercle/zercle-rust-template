# Technical Standards: Zercle Rust Template

## Technology Stack

### Core Framework
| Component | Technology | Version | Purpose |
|-----------|------------|---------|---------|
| Language | Rust | 1.75+ | Systems programming |
| Web Framework | Axum | 0.8 | HTTP server |
| Runtime | Tokio | 1 | Async runtime |
| Database | PostgreSQL | 14+ | Primary datastore |
| ORM/Query | SQLx | 0.8 | Type-safe SQL |

### Key Dependencies
| Dependency | Purpose |
|------------|---------|
| `tower` | Middleware and service composition |
| `tower-http` | HTTP-specific middleware (CORS, compression, trace) |
| `serde` | Serialization/deserialization |
| `chrono` | DateTime handling |
| `uuid` | UUID generation |
| `argon2` | Password hashing |
| `jsonwebtoken` | JWT signing/validation |
| `validator` | Input validation |
| `tracing` | Structured logging |
| `config` | Configuration management |

## Coding Standards

### Rust Idioms
- Use `?` operator for error propagation
- Prefer `match` for complex error handling
- Use `Into`/`From` traits for type conversions
- Leverage `Option` and `Result` exhaustive handling

### Naming Conventions
| Item | Convention | Example |
|------|------------|---------|
| Structs | PascalCase | `UserRepository` |
| Functions | snake_case | `find_by_id` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_CONNECTIONS` |
| Modules | snake_case | `user_service` |
| Traits | PascalCase | `Repository` |

### Error Handling
- Use `thiserror` for custom error types
- Use `anyhow` for application-level error handling
- Never use `.unwrap()` or `.expect()` in production code
- Provide context with `anyhow::Context`

### Code Organization
```
src/
├── lib.rs              # Public exports only
└── internal/           # Private implementation
    ├── domain/         # Business logic (no deps)
    │   ├── mod.rs
    │   └── user/
    │       ├── mod.rs
    │       ├── entity.rs
    │       └── refresh_token.rs
    └── infrastructure/ # Technical capabilities
        ├── mod.rs
        ├── config/
        └── db/
```

## Testing Standards

### Test Organization
```
tests/
├── unit/               # Unit tests
│   ├── mod.rs
│   └── password_test.rs
└── integration/        # Integration tests (future)
```

### Testing Principles
- Unit tests in same file as code (`#[cfg(test)]` modules)
- Integration tests in `tests/` directory
- Use `rstest` for parameterized tests
- Use `mockall` for mocking dependencies
- Use `wiremock` for HTTP mocking

### Coverage Requirements
- 80%+ coverage for domain layer
- 60%+ coverage for application layer
- Integration tests for all endpoints

## Security Standards

### Authentication
- Argon2id for password hashing
  - Memory: 19456 KB
  - Iterations: 2
  - Parallelism: 1
- JWT with configurable expiration
  - Access token: 1 hour default
  - Refresh token: 7 days default

### Input Validation
- Use `validator` crate for request validation
- Validate email format
- Enforce password complexity rules
- Sanitize all user inputs

### Security Headers
- CORS configuration per environment
- Security headers via tower-http

## Deployment Standards

### Configuration
| Environment | Source | Override |
|-------------|--------|----------|
| Local | `configs/local.yaml` | `.env` file |
| Dev | `configs/dev.yaml` | Environment variables |
| Prod | `configs/prod.yaml` | Environment variables |

### Environment Variables
All config keys can be overridden via env vars with `__` separator:
```
DATABASE__HOST=prod.db.com
DATABASE__PORT=5432
JWT__SECRET=secret-key
```

### Database Migrations
- Use `sqlx migrate` for migration management
- Migrations stored in `migrations/` directory
- Timestamp-based naming: `YYYYMMDDHHMMSS_description.sql`
- Always provide up and down migrations

## Performance Standards

### Database
- Connection pooling with configurable limits
- Use prepared statements (automatic with SQLx)
- Add indexes for query patterns
- Batch operations where possible

### Async Patterns
- Use `tokio::spawn` for CPU-bound tasks
- Use `Futures` for concurrent I/O
- Implement timeouts for external calls
- Use `parking_lot` for sync primitives

### Memory
- Minimize heap allocations in hot paths
- Use `&str` over `String` where possible
- Leverage stack allocation for small structs

## Documentation Standards

### Code Documentation
- Document all public APIs with rustdoc
- Include examples in doc comments
- Use `///` for item documentation
- Use `//!` for module documentation

### Architecture Documentation
- Document all architectural decisions in memory bank
- Include diagrams for complex flows
- Keep tech.md updated with new dependencies

## Git Standards

### Commit Messages
```
type(scope): subject

body (optional)

footer (optional)
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Example:
```
feat(auth): add user registration endpoint

- Implement UserService with validation
- Add password hashing with Argon2id
- Include integration tests
```

### Branching
- `main`: Production-ready code
- `develop`: Integration branch
- `feature/*`: New features
- `fix/*`: Bug fixes
- `hotfix/*`: Production fixes
