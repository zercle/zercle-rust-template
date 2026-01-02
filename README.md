# zercle-rust-template

A production-ready Rust template project using the Axum framework with PostgreSQL, featuring Domain-Driven Design (DDD) architecture and comprehensive infrastructure setup.

## Table of Contents

- [Project Overview](#project-overview)
- [Features](#features)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Development](#development)
- [Testing](#testing)
- [Deployment](#deployment)
- [Project Structure](#project-structure)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [License](#license)

## Project Overview

`zercle-rust-template` is a robust, production-ready Rust application template designed for building scalable web APIs. It combines the power of Rust's type safety with a clean, modular architecture that promotes maintainability and testability.

### Key Characteristics

- **Framework**: Axum 0.7 with Tower HTTP services
- **Database**: PostgreSQL with sqlx ORM and migrations
- **Architecture**: Domain-Driven Design (DDD) with clear separation of concerns
- **Authentication**: JWT-based authentication with Argon2 password hashing
- **Configuration**: Environment-based YAML configuration management
- **Runtime**: Async Tokio runtime for high-performance concurrency

## Features

### Core Features

- **RESTful API**: Clean HTTP API with proper routing and handlers
- **Authentication**: JWT token-based auth with secure password hashing
- **Authorization**: Middleware-based auth with rate limiting
- **Database**: PostgreSQL with connection pooling and health checks
- **Validation**: Request validation using the Validator crate

### Infrastructure

- **Logging**: Structured logging with tracing and JSON formatting
- **CORS**: Configurable Cross-Origin Resource Sharing
- **Rate Limiting**: Request throttling to prevent abuse
- **Compression**: Brotli compression for responses
- **Graceful Shutdown**: Proper signal handling for clean shutdowns

### Developer Experience

- **Docker Support**: Containerized deployment with Docker Compose
- **Makefile**: Comprehensive build automation
- **Testing**: Unit and integration test suite
- **Code Quality**: Clippy linter, rustfmt, and cargo-audit
- **Hot Reload**: cargo-watch for development

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Layer                         │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                      HTTP Server (Axum)                    │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐  │  │
│  │  │ Routes  │ │Handlers │ │Middleware│ │   OpenAPI/Swagger│ │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Infrastructure Layer                      │
│  ┌──────────────┐ ┌──────────────┐ ┌────────────────────────┐  │
│  │    HTTP      │ │   Database   │ │    Configuration       │  │
│  │  (axum/tower)│ │   (sqlx)     │ │    (config crate)      │  │
│  └──────────────┘ └──────────────┘ └────────────────────────┘  │
│  ┌──────────────┐ ┌──────────────┐ ┌────────────────────────┐  │
│  │  Middleware  │ │  Migrations  │ │    Logging/Tracing     │  │
│  │(CORS, Auth)  │ │  (sqlx-cli)  │ │    (tracing)           │  │
│  └──────────────┘ └──────────────┘ └────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Domain Layer                            │
│  ┌────────────────┐ ┌────────────────┐ ┌─────────────────────┐  │
│  │    Entities    │ │   Repositories │ │     Use Cases       │  │
│  │  (User, Task)  │ │   (Interface)  │ │  (Business Logic)   │  │
│  └────────────────┘ └────────────────┘ └─────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Layer Descriptions

- **Domain Layer**: Pure Rust business logic without external dependencies
- **Infrastructure Layer**: Database, HTTP servers, and external services
- **Application Layer**: Orchestration of domain logic and infrastructure

## Prerequisites

- **Rust**: Version 1.75 or higher (see [`rust-toolchain.toml`](rust-toolchain.toml))
- **PostgreSQL**: Version 14 or higher
- **Docker & Docker Compose**: For containerized development
- **sqlx-cli**: For database migrations (`cargo install sqlx-cli`)
- **Make**: For build automation

## Installation

### 1. Clone and Setup

```bash
git clone <repository-url>
cd zercle-rust-template
```

### 2. Install Dependencies

```bash
# Using make
make setup

# Or manually
cargo fetch
sqlx-cli install
```

### 3. Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit configuration
nano .env
```

### 4. Setup Database

```bash
# Run migrations
make db-migrate

# Seed with sample data (optional)
make db-seed
```

## Quick Start

### Development Mode

```bash
# Start with default environment (dev)
make run

# Or with specific environment
ENV=local make run
```

### Docker Mode

```bash
# Build and start all services
make docker-up

# View logs
make docker-logs

# Stop services
make docker-down
```

### Verify Installation

```bash
# Health check
curl http://localhost:3000/health

# API should respond with service status
```

## Development

### Available Make Commands

```bash
# Build
make build          # Debug build
make build-release  # Optimized release build

# Run
make run            # Run application
make run-dev        # Run in dev mode
make run-local      # Run in local mode
make run-uat        # Run in UAT mode
make run-prod       # Run in production mode

# Database
make db-migrate     # Run migrations
make db-seed        # Seed database
make db-reset       # Reset database
make db-generate    # Generate sqlc code

# Testing
make test           # Run all tests
make test-unit      # Unit tests only
make test-integration # Integration tests only
make test-coverage  # Tests with coverage

# Code Quality
make fmt            # Format code
make fmt-check      # Check formatting
make clippy         # Run linter
make lint           # Run all linters
make audit          # Security audit

# Development
make watch          # Hot reload development
make watch-test     # Watch and test
make doc            # Generate docs
make help           # Show all commands
```

### Development Scripts

```bash
# Run development server with hot reload
./scripts/run-dev.sh

# Seed database with test data
./scripts/seed-db.sh
```

### Database Migrations

Migrations are located in [`sqlc/migrations/`](sqlc/migrations/) and can be managed with:

```bash
# Create new migration
cargo sqlx migrate add -r <migration_name>

# Run migrations
make db-migrate

# Rollback migrations
cargo sqlx migrate revert
```

## Testing

### Running Tests

```bash
# Run all tests
make test

# Unit tests only
make test-unit

# Integration tests only
make test-integration

# With coverage report
make test-coverage
```

### Test Structure

```
tests/
├── integration/
│   └── api_test.rs      # API integration tests
└── unit/
    ├── entity_validation_test.rs  # Entity validation tests
    └── usecase_test.rs    # Use case tests
```

### Example Test

```rust
#[tokio::test]
async fn test_create_user() {
    let app = spawn_app().await;
    let response = app.post_users("/users", &body).await;
    assert_eq!(response.status(), 201);
}
```

## Deployment

### Docker Deployment

#### Production Build

```bash
# Build Docker image
make docker-build

# Start production services
docker-compose -f deployments/docker/docker-compose.yml up -d
```

#### Environment Variables

Configure via environment variables or [`configs/prod.yaml`](configs/prod.yaml):

```yaml
server:
  port: 3000
  host: "0.0.0.0"
  env: "prod"

database:
  host: "postgres"
  port: 5432
  user: "postgres"
  password: "${DB_PASSWORD}"
  dbname: "postgres"

jwt:
  secret: "${JWT_SECRET}"
  expiration: 3600
```

### Kubernetes Deployment

The Docker image can be deployed to any Kubernetes cluster:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: zercle-rust-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: zercle-rust-api
  template:
    spec:
      containers:
      - name: api
        image: zercle-rust-template:latest
        ports:
        - containerPort: 3000
```

### Health Checks

The application exposes the following health endpoints:

- `/health` - Overall health check
- `/health/live` - Liveness probe
- `/health/ready` - Readiness probe

## Project Structure

```
zercle-rust-template/
├── Cargo.toml              # Project manifest
├── Cargo.lock              # Locked dependencies
├── Makefile                # Build automation
├── rust-toolchain.toml     # Rust version specification
├── sqlc.yaml              # SQLx configuration
│
├── configs/               # Configuration files
│   ├── dev.yaml          # Development settings
│   ├── local.yaml        # Local environment
│   ├── prod.yaml         # Production settings
│   └── uat.yaml          # User acceptance testing
│
├── deployments/           # Docker configurations
│   └── docker/
│       ├── Dockerfile
│       └── docker-compose.yml
│
├── scripts/               # Utility scripts
│   ├── run-dev.sh
│   └── seed-db.sh
│
├── sqlc/                  # Database layer
│   ├── migrations/       # Database migrations
│   └── queries/          # SQL queries
│
├── src/                   # Source code
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Library root
│   ├── app.rs            # Application builder
│   │
│   ├── config/           # Configuration
│   │   ├── mod.rs
│   │   └── settings.rs
│   │
│   ├── domain/           # Business logic
│   │   ├── entities/     # Domain entities
│   │   │   ├── user.rs
│   │   │   └── task.rs
│   │   ├── repositories/ # Repository interfaces
│   │   │   ├── user_repository.rs
│   │   │   └── task_repository.rs
│   │   └── usecases/     # Business logic
│   │       ├── user_usecase.rs
│   │       └── task_usecase.rs
│   │
│   └── infrastructure/   # External services
│       ├── config/
│       ├── db/           # Database implementation
│       │   ├── connection.rs
│       │   ├── migrations.rs
│       │   └── postgres_repository.rs
│       ├── http/         # HTTP server
│       │   ├── handlers.rs
│       │   ├── routes.rs
│       │   └── server.rs
│       └── middleware/   # HTTP middleware
│           ├── auth.rs
│           ├── cors.rs
│           ├── logging.rs
│           └── rate_limit.rs
│
└── tests/                # Test suite
    ├── integration/
    └── unit/
```

## Configuration

### Configuration Files

| File | Environment | Description |
|------|-------------|-------------|
| [`configs/dev.yaml`](configs/dev.yaml) | Development | Development settings with debug logging |
| [`configs/local.yaml`](configs/local.yaml) | Local | Local development environment |
| [`configs/uat.yaml`](configs/uat.yaml) | UAT | User acceptance testing |
| [`configs/prod.yaml`](configs/prod.yaml) | Production | Production settings |

### Configuration Structure

```yaml
server:
  port: 3000
  host: "0.0.0.0"
  env: "dev"

database:
  host: "localhost"
  port: 5432
  user: "postgres"
  password: "postgres"
  dbname: "postgres"
  max_conns: 25
  min_conns: 5

jwt:
  secret: "your-jwt-secret"
  expiration: 3600

logging:
  level: "debug"
  format: "json"

cors:
  allowed_origins:
    - "http://localhost:3000"

rate_limit:
  requests: 100
  window: 60
```

### Environment Variables

Override configuration with environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_ENV` | Environment name | `dev` |
| `SERVER_PORT` | HTTP server port | `3000` |
| `DB_HOST` | Database host | `localhost` |
| `DB_PORT` | Database port | `5432` |
| `DB_USER` | Database user | `postgres` |
| `DB_PASSWORD` | Database password | `postgres` |
| `DB_NAME` | Database name | `postgres` |
| `JWT_SECRET` | JWT signing secret | - |
| `LOG_LEVEL` | Logging level | `info` |

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Standards

- Follow Rust idioms and best practices
- Run `make lint` before committing
- Write tests for new functionality
- Update documentation as needed
- Use conventional commit messages

## License

This project is licensed under the MIT License - see the [`LICENSE.md`](LICENSE.md) file for details.

---

**Built with ❤️ using Rust, Axum, and PostgreSQL**
