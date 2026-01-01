# Zercle Rust Template

A production-ready RESTful API template built with Rust Axum framework, featuring clean architecture, JWT authentication, and PostgreSQL database. This template provides a solid foundation for building Rust microservices or REST APIs with best practices already implemented.

## Features

- **Clean Architecture** - Domain-driven design with clear separation of concerns
- **Type-safe Database Operations** - sqlx with compile-time checked queries
- **JWT Authentication** - Stateless authentication with configurable expiration
- **Password Security** - Argon2id hashing for secure password storage
- **Comprehensive Testing** - Unit, integration, and mock testing infrastructure
- **API Documentation** - OpenAPI/Swagger documentation out of the box with utoipa
- **Structured Logging** - Tracing for async-aware JSON logging
- **Docker Support** - Containerized deployment with Docker Compose
- **Rate Limiting** - Configurable request rate limiting
- **CORS Support** - Configurable cross-origin resource sharing
- **Health Checks** - Application and readiness endpoints
- **Memory Safety** - Rust's ownership system guarantees memory safety
- **Zero-Cost Abstractions** - Performance without runtime overhead
- **Async/Await** - Efficient async I/O with Tokio

## Tech Stack

- **Language**: Rust 1.80+
- **Web Framework**: Axum 0.7+
- **Database**: PostgreSQL 12+ with sqlx driver
- **Query Builder**: sqlx (compile-time checked queries)
- **Authentication**: JWT (jsonwebtoken crate)
- **Password Hashing**: Argon2id (argon2 crate)
- **Configuration**: config crate with serde
- **Logging**: tracing and tracing-subscriber
- **Validation**: validator or garde
- **Documentation**: utoipa (OpenAPI/Swagger)
- **Testing**: rstest, mockall, testcontainers-rs
- **Async Runtime**: Tokio

## Project Structure

```
.
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library entry point
│   ├── app/
│   │   └── mod.rs              # Application orchestration & DI
│   ├── domain/
│   │   ├── user/               # User domain (example)
│   │   │   ├── entity.rs       # Business entities
│   │   │   ├── handler.rs      # HTTP handlers
│   │   │   ├── repository.rs   # Data access layer
│   │   │   ├── service.rs      # Business logic
│   │   │   ├── request.rs      # Request DTOs
│   │   │   ├── response.rs     # Response DTOs
│   │   │   ├── mock.rs         # Mock implementations
│   │   │   └── mod.rs         # Domain module
│   │   └── task/               # Task domain (example)
│   ├── infrastructure/
│   │   ├── config/             # Configuration management
│   │   ├── db/                 # Database abstraction
│   │   ├── http/               # HTTP client
│   │   ├── logger/             # Structured logging
│   │   └── password/           # Password hashing
│   ├── middleware/             # Custom middleware
│   └── health/                 # Health checks
├── migrations/                 # Database migrations
├── configs/
│   ├── local.toml              # Local development config
│   ├── dev.toml                # Development config
│   ├── uat.toml                # UAT config
│   └── prod.toml               # Production config
├── tests/                      # Integration tests
├── scripts/
│   ├── run-dev.sh              # Development runner
│   └── seed-db.sh              # Database seeding
├── deployments/
│   └── docker/
│       ├── Dockerfile           # Docker image
│       └── docker-compose.yml   # Docker Compose setup
├── docs/                       # OpenAPI documentation
├── .env.example                # Environment variables template
├── Cargo.toml                  # Rust package definition
├── Cargo.lock                  # Rust dependencies lock file
├── Makefile                    # Common operations
└── .clippy.toml               # Linting configuration
```

## Architecture

This template follows **Clean Architecture** with **Domain-Driven Design (DDD)** principles:

### Layers

1. **Domain Layer** (`src/domain/`) - Core business logic and entities, independent of infrastructure
2. **Infrastructure Layer** (`src/infrastructure/`) - External concerns and technical implementations
3. **Application Layer** (`src/app/`) - Application orchestration and dependency injection
4. **Entry Point** (`src/main.rs`) - Application bootstrap

### Data Flow

```
Client → Handler → Service → Repository → Database
          ↓         ↓          ↓
      Request   Business    Data Access
      DTO       Logic       Layer
```

### Rust-Specific Patterns

- **Traits** for interfaces and dependency injection
- **Arc<T>** for shared state across async tasks
- **Result<T, E>** for error handling
- **Async/Await** with Tokio runtime
- **Ownership and Borrowing** for memory safety

## Getting Started

### Prerequisites

- Rust 1.80 or higher (install via [rustup](https://rustup.rs/))
- PostgreSQL 12+
- Docker (optional, for containerized deployment)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/zercle/zercle-rust-template.git
cd zercle-rust-template
```

2. Copy environment variables:
```bash
cp .env.example .env
```

3. Install dependencies:
```bash
cargo build
```

4. Configure database connection in `.env` or `configs/local.toml`

5. Run database migrations:
```bash
sqlx migrate run
```

6. Run the application:
```bash
cargo run
```

### Running the Application

#### Development

```bash
# Set environment
export APP_ENV=local

# Run the application
cargo run

# Or with hot reload (install cargo-watch)
cargo install cargo-watch
cargo watch -x run
```

#### Production

```bash
# Build the binary
cargo build --release

# Run the binary
./target/release/zercle-rust-template
```

#### Docker

```bash
# Build Docker image
docker build -t zercle-rust-template .

# Run container
docker run -p 3000:3000 \
  -e APP_ENV=prod \
  -e DATABASE_URL=postgres://user:pass@host:5432/dbname \
  zercle-rust-template
```

#### Docker Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down
```

## Configuration

Configuration is managed through TOML files in the `configs/` directory:

- `local.toml` - Local development
- `dev.toml` - Development environment
- `uat.toml` - User acceptance testing
- `prod.toml` - Production

Environment variables can override configuration values. Set `APP_ENV` to select the configuration file.

### Key Configuration Sections

- **Database**: Connection string, pool settings
- **JWT**: Secret key, expiration time
- **Server**: Port, timeout settings
- **Logging**: Level, format, output
- **CORS**: Allowed origins, methods, headers
- **Rate Limiting**: Requests per window, window duration

## API Documentation

Once the application is running, access the Swagger documentation at:

```
http://localhost:3000/swagger
```

### Available Endpoints

#### Health Checks
- `GET /health` - Application health check
- `GET /readiness` - Readiness probe

#### Authentication
- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/login` - User login

#### User Management
- `GET /api/v1/users` - List users (paginated)
- `GET /api/v1/users/:id` - Get user profile
- `PUT /api/v1/users/:id` - Update user profile (protected)
- `DELETE /api/v1/users/:id` - Delete account (protected)

#### Task Management (Example Domain)
- `POST /api/v1/tasks` - Create task (protected)
- `GET /api/v1/tasks` - List tasks (protected, paginated)
- `GET /api/v1/tasks/:id` - Get task (protected)
- `PUT /api/v1/tasks/:id` - Update task (protected)
- `DELETE /api/v1/tasks/:id` - Delete task (protected)

## Testing

### Run All Tests

```bash
cargo test
```

### Run Tests with Coverage

```bash
cargo tarpaulin --out Html
```

Or using llvm-cov:

```bash
cargo llvm-cov --html
```

### Run Integration Tests

```bash
cargo test --test integration
```

### Run Specific Test

```bash
cargo test test_login_valid_credentials
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

## Development Guidelines

### Adding a New Domain

1. Create domain structure under `src/domain/<domain>/`
2. Define entity in `entity.rs`
3. Create traits in `mod.rs`
4. Implement repository, service, and handler
5. Add request/response DTOs
6. Write tests
7. Wire dependencies in `src/app/mod.rs`
8. Register routes
9. Update OpenAPI documentation with utoipa

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use `cargo clippy` for linting
- Write rustdoc comments for public items
- Keep functions under 50 lines when possible
- Follow SOLID principles
- Use snake_case for functions and variables
- Use PascalCase for types and traits

### Testing Standards

- Write unit tests for business logic
- Use table-driven tests with rstest for multiple scenarios
- Mock external dependencies with mockall
- Aim for >80% coverage on critical paths
- Test error paths, not just happy paths
- Use `#[tokio::test]` for async tests

### Database Migrations

1. Create migration files: `sqlx migrate add -r description`
2. Format: `YYYYMMDD_NNN_description`
3. Write both up and down migrations
4. Apply migrations: `sqlx migrate run`
5. Verify: `sqlx migrate info`

## Common Commands

### Linting

```bash
# Run clippy
cargo clippy

# Fix issues automatically
cargo clippy --fix
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

# Update dependencies
cargo update

# Check for outdated
cargo outdated

# Remove dependency
cargo remove <crate>
```

### Documentation

```bash
# Generate and open documentation
cargo doc --open

# Document private items
cargo doc --document-private-items
```

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Check without building
cargo check
```

## Environment Variables

Key environment variables (see `.env.example`):

- `APP_ENV` - Environment (local, dev, uat, prod)
- `DATABASE_URL` - PostgreSQL connection string
- `JWT_SECRET` - JWT signing secret
- `JWT_EXPIRATION` - Token expiration time
- `SERVER_PORT` - Server port (default: 3000)
- `RUST_LOG` - Log level (trace, debug, info, warn, error)

## Security

- Passwords hashed with Argon2id
- JWT tokens for stateless authentication
- Input validation on all endpoints
- CORS configuration per environment
- Rate limiting to prevent abuse
- SQL injection prevention via sqlx
- Memory safety guarantees from Rust
- No null pointer dereferences
- No data races in safe Rust

## Performance

- Database connection pooling with sqlx
- Efficient query generation with compile-time checking
- Structured logging with minimal overhead
- Graceful shutdown handling with Tokio
- Configurable timeouts
- Zero-cost abstractions
- Async/await with Tokio for efficient I/O
- Memory safety without garbage collection

## Deployment

### Production Checklist

- [ ] Set strong JWT secret
- [ ] Configure production database
- [ ] Enable HTTPS/TLS
- [ ] Set appropriate CORS origins
- [ ] Configure rate limiting
- [ ] Set log level to INFO or WARN
- [ ] Enable health checks
- [ ] Configure monitoring and alerting
- [ ] Run database migrations
- [ ] Test all endpoints
- [ ] Build with `--release` flag
- [ ] Run `cargo audit` for security vulnerabilities

### Docker Deployment

The provided Dockerfile uses a multi-stage build for optimization:

- Builder stage: Compiles the Rust binary
- Runtime stage: Alpine-based minimal image
- Non-root user for security
- Health checks configured
- Optimized with cargo-chef for better caching

### Rust-Specific Deployment Considerations

- Use `cargo build --release` for production builds
- Enable LTO in Cargo.toml for optimization
- Strip symbols for smaller binaries
- Use `cargo-chef` for better Docker caching
- Consider static linking with musl target
- Set `RUST_LOG` environment variable for logging

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Pull Request Guidelines

- Follow existing code style
- Add tests for new features
- Update documentation
- Ensure all tests pass (`cargo test`)
- Run linter and fix issues (`cargo clippy`)
- Format code (`cargo fmt`)

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.

## Support

For issues, questions, or contributions, please visit the GitHub repository.

## Roadmap

Future enhancements planned:

- [ ] Redis caching layer
- [ ] Message queue integration (RabbitMQ/Kafka with lapin or rdkafka)
- [ ] Metrics collection (Prometheus with prometheus-client)
- [ ] Distributed tracing (OpenTelemetry)
- [ ] API versioning strategy
- [ ] GraphQL support option (juniper or async-graphql)
- [ ] gRPC support (tonic)
- [ ] Additional example domains

## Acknowledgments

Built with best practices and modern Rust development tools. Special thanks to the open-source community for the excellent libraries and frameworks used in this project.

### Key Libraries

- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [sqlx](https://github.com/launchbadge/sqlx) - Database toolkit
- [Tokio](https://tokio.rs/) - Async runtime
- [Serde](https://serde.rs/) - Serialization framework
- [Tracing](https://github.com/tokio-rs/tracing) - Instrumentation
- [Utoipa](https://github.com/juhaku/utoipa) - OpenAPI documentation
- [jsonwebtoken](https://github.com/Keats/jsonwebtoken) - JWT library
- [Argon2](https://github.com/RustCrypto/password-hashes) - Password hashing

## Rust Advantages

This template leverages Rust's unique advantages:

- **Memory Safety**: No null pointer dereferences, no buffer overflows, no data races
- **Zero-Cost Abstractions**: High-level abstractions with no runtime overhead
- **Fearless Concurrency**: Safe parallel programming without data races
- **Modern Tooling**: Cargo package manager, excellent compiler error messages
- **Performance**: Comparable to C/C++ without the safety trade-offs
- **Ecosystem**: Growing ecosystem with high-quality crates
- **Cross-Platform**: Write once, run anywhere
- **WebAssembly**: Compile to Wasm for web deployment
