# =============================================================================
# Zercle Rust Template - Makefile
# =============================================================================
# A comprehensive Makefile for Rust project development
#
# Usage:
#   make <target>
#   ENV=<env> make <target>  # Override environment (default: dev)
#
# Environments: dev, local, uat, prod
# =============================================================================

# -----------------------------------------------------------------------------
# Variables
# -----------------------------------------------------------------------------

# Project metadata
PROJECT_NAME := $(shell cat Cargo.toml | grep '^name' | head -1 | cut -d'"' -f2)
PROJECT_VERSION := $(shell cat Cargo.toml | grep '^version' | head -1 | cut -d'"' -f2)

# Rust toolchain
CARGO := cargo
RUSTC := rustc
RUSTFMT := rustfmt
RUSTUP := rustup

# Build flags
DEBUG_BUILD_DIR := target/debug
RELEASE_BUILD_DIR := target/release

# Environment configuration
ENV ?= dev
CONFIG_FILE := configs/$(ENV).yaml

# Colors for output (with fallbacks for terminals that don't support colors)
ifeq ($(shell tput colors 2>/dev/null || echo 0),0)
    NORMAL := 
    BOLD := 
    RED := 
    GREEN := 
    YELLOW := 
    BLUE := 
else
    NORMAL := $(shell tput sgr0)
    BOLD := $(shell tput bold)
    RED := $(shell tput setaf 1)
    GREEN := $(shell tput setaf 2)
    YELLOW := $(shell tput setaf 3)
    BLUE := $(shell tput setaf 4)
endif

# Default environment variables for different targets
DEV_ENV_VARS := DATABASE_URL=postgres://postgres:postgres@localhost:5432/zercle_dev
LOCAL_ENV_VARS := DATABASE_URL=postgres://postgres:postgres@localhost:5432/zercle_local
UAT_ENV_VARS := DATABASE_URL=postgres://postgres:postgres@localhost:5432/zercle_uat
PROD_ENV_VARS := DATABASE_URL=postgres://postgres:postgres@localhost:5432/zercle_prod

# Docker compose files
DOCKER_COMPOSE_FILE := deployments/docker/docker-compose.yml
DOCKER_COMPOSE_TEST_FILE := deployments/docker/docker-compose.test.yml

# sqlc configuration
SQLC := sqlc
SQLC_CONFIG := sqlc.yaml

# -----------------------------------------------------------------------------
# Phony targets (always run)
# -----------------------------------------------------------------------------
.PHONY: all build build-release rebuild \
    run run-dev run-local run-uat run-prod \
    test test-unit test-integration test-coverage \
    db-migrate db-seed db-reset \
    fmt fmt-check clippy lint audit \
    docker-build docker-up docker-down docker-test \
    clean clean-all \
    watch setup help

# -----------------------------------------------------------------------------
# Build Targets
# -----------------------------------------------------------------------------

# Build the project in debug mode
build:
	@echo "$(BLUE)[BUILD]$(NORMAL) Building $(PROJECT_NAME) v$(PROJECT_VERSION) in debug mode..."
	$(CARGO) build

# Build the project in release mode
build-release:
	@echo "$(BLUE)[BUILD]$(NORMAL) Building $(PROJECT_NAME) v$(PROJECT_VERSION) in release mode..."
	$(CARGO) build --release
	@echo "$(GREEN)[BUILD]$(NORMAL) Release binary available at: $(RELEASE_BUILD_DIR)/$(PROJECT_NAME)"

# Clean and rebuild
rebuild: clean build

# -----------------------------------------------------------------------------
# Run Targets
# -----------------------------------------------------------------------------

# Run the application (uses ENV variable, default: dev)
run: run-$(ENV)

# Run with dev environment
run-dev:
	@echo "$(BLUE)[RUN]$(NORMAL) Starting $(PROJECT_NAME) in dev mode..."
	@if [ ! -f $(CONFIG_FILE) ]; then \
		echo "$(RED)[ERROR]$(NORMAL) Config file not found: $(CONFIG_FILE)"; \
		echo "$(YELLOW)[INFO]$(NORMAL) Copy .env.example to .env and configure it"; \
		exit 1; \
	fi
	$(DEV_ENV_VARS) $(CARGO) run

# Run with local environment
run-local:
	@echo "$(BLUE)[RUN]$(NORMAL) Starting $(PROJECT_NAME) in local mode..."
	@if [ ! -f $(CONFIG_FILE) ]; then \
		echo "$(RED)[ERROR]$(NORMAL) Config file not found: $(CONFIG_FILE)"; \
		exit 1; \
	fi
	$(LOCAL_ENV_VARS) $(CARGO) run

# Run with UAT environment
run-uat:
	@echo "$(BLUE)[RUN]$(NORMAL) Starting $(PROJECT_NAME) in UAT mode..."
	@if [ ! -f $(CONFIG_FILE) ]; then \
		echo "$(RED)[ERROR]$(NORMAL) Config file not found: $(CONFIG_FILE)"; \
		exit 1; \
	fi
	$(UAT_ENV_VARS) $(CARGO) run

# Run with prod environment
run-prod:
	@echo "$(RED)[RUN]$(NORMAL) Starting $(PROJECT_NAME) in PROD mode..."
	@if [ ! -f $(CONFIG_FILE) ]; then \
		echo "$(RED)[ERROR]$(NORMAL) Config file not found: $(CONFIG_FILE)"; \
		exit 1; \
	fi
	$(PROD_ENV_VARS) $(CARGO) run --release

# -----------------------------------------------------------------------------
# Test Targets
# -----------------------------------------------------------------------------

# Run all tests
test:
	@echo "$(BLUE)[TEST]$(NORMAL) Running all tests..."
	$(CARGO) test

# Run unit tests only
test-unit:
	@echo "$(BLUE)[TEST]$(NORMAL) Running unit tests..."
	$(CARGO) test --lib

# Run integration tests only
test-integration:
	@echo "$(BLUE)[TEST]$(NORMAL) Running integration tests..."
	$(CARGO) test --tests integration

# Run tests with coverage
test-coverage:
	@echo "$(BLUE)[TEST]$(NORMAL) Running tests with coverage..."
	@if ! command -v tarpaulin &> /dev/null; then \
		echo "$(YELLOW)[INFO]$(NORMAL) Installing cargo-tarpaulin..."; \
		$(CARGO) install cargo-tarpaulin; \
	fi
	$(CARGO) tarpaulin --out Html

# -----------------------------------------------------------------------------
# Database Targets
# -----------------------------------------------------------------------------

# Run database migrations
db-migrate:
	@echo "$(BLUE)[DB]$(NORMAL) Running database migrations..."
	@if [ ! -f $(SQLC_CONFIG) ]; then \
		echo "$(RED)[ERROR]$(NORMAL) sqlc config not found: $(SQLC_CONFIG)"; \
		exit 1; \
	fi
	$(CARGO) run --bin sqlx-cli -- migrate run

# Seed database with test data
db-seed:
	@echo "$(BLUE)[DB]$(NORMAL) Seeding database..."
	@if [ -f scripts/seed-db.sh ]; then \
		bash scripts/seed-db.sh; \
	else \
		echo "$(YELLOW)[WARN]$(NORMAL) Seed script not found: scripts/seed-db.sh"; \
	fi

# Reset database (migrate + seed)
db-reset:
	@echo "$(BLUE)[DB]$(NORMAL) Resetting database..."
	$(CARGO) run --bin sqlx-cli -- migrate drop -y
	$(CARGO) run --bin sqlx-cli -- migrate run
	@echo "$(BLUE)[DB]$(NORMAL) Database reset complete. Seeding data..."
	@if [ -f scripts/seed-db.sh ]; then \
		bash scripts/seed-db.sh; \
	fi

# Generate sqlc code
db-generate:
	@echo "$(BLUE)[DB]$(NORMAL) Generating sqlc code..."
	$(SQLC) generate

# -----------------------------------------------------------------------------
# Code Quality Targets
# -----------------------------------------------------------------------------

# Format code with rustfmt
fmt:
	@echo "$(BLUE)[FMT]$(NORMAL) Formatting code..."
	$(CARGO) fmt

# Check code formatting
fmt-check:
	@echo "$(BLUE)[FMT]$(NORMAL) Checking code formatting..."
	$(CARGO) fmt -- --check

# Run clippy linter
clippy:
	@echo "$(BLUE)[CLIPPY]$(NORMAL) Running clippy linter..."
	$(CARGO) clippy

# Run all linters (fmt-check + clippy)
lint: fmt-check clippy

# Run cargo audit for security vulnerabilities
audit:
	@echo "$(BLUE)[AUDIT]$(NORMAL) Checking for security vulnerabilities..."
	@if ! command -v cargo-audit &> /dev/null; then \
		echo "$(YELLOW)[INFO]$(NORMAL) Installing cargo-audit..."; \
		$(CARGO) install cargo-audit; \
	fi
	cargo-audit audit

# Check for outdated dependencies
outdated:
	@echo "$(BLUE)[OUTDATED]$(NORMAL) Checking for outdated dependencies..."
	@if ! command -v cargo-outdated &> /dev/null; then \
		echo "$(YELLOW)[INFO]$(NORMAL) Installing cargo-outdated..."; \
		$(CARGO) install cargo-outdated; \
	fi
	cargo-outdated

# -----------------------------------------------------------------------------
# Docker Targets
# -----------------------------------------------------------------------------

# Build Docker image
docker-build:
	@echo "$(BLUE)[DOCKER]$(NORMAL) Building Docker image..."
	docker build -t $(PROJECT_NAME):latest -f deployments/docker/Dockerfile .

# Start Docker containers
docker-up:
	@echo "$(BLUE)[DOCKER]$(NORMAL) Starting Docker containers..."
	docker-compose -f $(DOCKER_COMPOSE_FILE) up -d

# Stop Docker containers
docker-down:
	@echo "$(BLUE)[DOCKER]$(NORMAL) Stopping Docker containers..."
	docker-compose -f $(DOCKER_COMPOSE_FILE) down

# View Docker logs
docker-logs:
	@echo "$(BLUE)[DOCKER]$(NORMAL) Showing Docker logs..."
	docker-compose -f $(DOCKER_COMPOSE_FILE) logs -f

# Run tests in Docker
docker-test:
	@echo "$(BLUE)[DOCKER]$(NORMAL) Running tests in Docker..."
	docker-compose -f $(DOCKER_COMPOSE_TEST_FILE) up --build --abort-on-container-exit

# Build and push Docker image (for CI/CD)
docker-publish:
	@echo "$(BLUE)[DOCKER]$(NORMAL) Building and publishing Docker image..."
	@read -p "Enter Docker registry URL (e.g., ghcr.io/username): " REGISTRY; \
	read -p "Enter image tag (default: latest): " TAG; \
	[ -z "$$TAG" ] && TAG="latest"; \
	docker build -t $$REGISTRY/$(PROJECT_NAME):$$TAG -f deployments/docker/Dockerfile .; \
	docker push $$REGISTRY/$(PROJECT_NAME):$$TAG; \
	echo "$(GREEN)[DOCKER]$(NORMAL) Image published: $$REGISTRY/$(PROJECT_NAME):$$TAG"

# -----------------------------------------------------------------------------
# Clean Targets
# -----------------------------------------------------------------------------

# Clean build artifacts
clean:
	@echo "$(BLUE)[CLEAN]$(NORMAL) Cleaning build artifacts..."
	$(CARGO) clean
	@echo "$(GREEN)[CLEAN]$(NORMAL) Build artifacts cleaned"

# Clean everything including Docker
clean-all: clean
	@echo "$(BLUE)[CLEAN]$(NORMAL) Cleaning Docker containers and volumes..."
	-docker-compose -f $(DOCKER_COMPOSE_FILE) down -v 2>/dev/null || true
	@echo "$(GREEN)[CLEAN]$(NORMAL) All cleaned"

# Clean cargo cache
clean-cache:
	@echo "$(BLUE)[CLEAN]$(NORMAL) Cleaning cargo cache..."
	rm -rf ~/.cargo/registry/index/*
	rm -rf ~/.cargo/registry/cache/*
	rm -rf ~/.cargo/git/db/*
	@echo "$(GREEN)[CLEAN]$(NORMAL) Cargo cache cleaned"

# -----------------------------------------------------------------------------
# Development Helpers
# -----------------------------------------------------------------------------

# Watch for changes and rebuild
watch:
	@echo "$(BLUE)[WATCH]$(NORMAL) Watching for changes..."
	@if ! command -v cargo-watch &> /dev/null; then \
		echo "$(YELLOW)[INFO]$(NORMAL) Installing cargo-watch..."; \
		$(CARGO) install cargo-watch; \
	fi
	cargo watch -x run

# Watch and run tests
watch-test:
	@echo "$(BLUE)[WATCH]$(NORMAL) Watching for changes and running tests..."
	@if ! command -v cargo-watch &> /dev/null; then \
		$(CARGO) install cargo-watch; \
	fi
	cargo watch -x test

# Initial project setup
setup:
	@echo "$(BLUE)[SETUP]$(NORMAL) Setting up project..."
	@if [ ! -f .env ]; then \
		echo "$(YELLOW)[INFO]$(NORMAL) Creating .env from .env.example..."; \
		cp .env.example .env; \
		echo "$(GREEN)[SETUP]$(NORMAL) Please configure .env with your settings"; \
	else \
		echo "$(YELLOW)[INFO]$(NORMAL) .env already exists"; \
	fi
	@echo "$(BLUE)[SETUP]$(NORMAL) Installing dependencies..."
	$(CARGO) fetch
	@echo "$(BLUE)[SETUP]$(NORMAL) Verifying sqlc configuration..."
	@if [ -f $(SQLC_CONFIG) ]; then \
		$(SQLC) version; \
	fi
	@echo "$(GREEN)[SETUP]$(NORMAL) Project setup complete!"

# Generate documentation
doc:
	@echo "$(BLUE)[DOC]$(NORMAL) Generating documentation..."
	$(CARGO) doc --no-deps
	@echo "$(GREEN)[DOC]$(NORMAL) Documentation generated in target/doc/"

# Open documentation in browser
doc-open:
	@echo "$(BLUE)[DOC]$(NORMAL) Opening documentation..."
	@if command -v open &> /dev/null; then \
		open target/doc/$(PROJECT_NAME)/index.html; \
	elif command -v xdg-open &> /dev/null; then \
		xdg-open target/doc/$(PROJECT_NAME)/index.html; \
	else \
		echo "$(YELLOW)[INFO]$(NORMAL) Documentation generated at target/doc/$(PROJECT_NAME)/index.html"; \
	fi

# Open shell in Docker container
docker-shell:
	@echo "$(BLUE)[DOCKER]$(NORMAL) Opening shell in application container..."
	docker-compose -f $(DOCKER_COMPOSE_FILE) exec app sh

# Check project dependencies
deps:
	@echo "$(BLUE)[DEPS]$(NORMAL) Checking project dependencies..."
	$(CARGO) tree -e no-dev
	@echo ""
	$(CARGO) update --dry-run

# -----------------------------------------------------------------------------
# Help Target
# -----------------------------------------------------------------------------

# Display available targets
help:
	@echo ""
	@echo "$(BOLD)================================================================================$(NORMAL)"
	@echo "$(BOLD)  $(PROJECT_NAME) v$(PROJECT_VERSION) - Makefile Help$(NORMAL)"
	@echo "$(BOLD)================================================================================$(NORMAL)"
	@echo ""
	@echo "$(BOLD)Usage:$(NORMAL)"
	@echo "  make <target>           Run a specific target"
	@echo "  ENV=<env> make <target> Override environment (dev, local, uat, prod)"
	@echo ""
	@echo "$(BOLD)Build Targets:$(NORMAL)"
	@echo "  build           Build in debug mode"
	@echo "  build-release   Build in release mode"
	@echo "  rebuild         Clean and rebuild"
	@echo ""
	@echo "$(BOLD)Run Targets:$(NORMAL)"
	@echo "  run         Run application (default: dev)"
	@echo "  run-dev     Run with dev environment"
	@echo "  run-local   Run with local environment"
	@echo "  run-uat     Run with UAT environment"
	@echo "  run-prod    Run with prod environment"
	@echo ""
	@echo "$(BOLD)Test Targets:$(NORMAL)"
	@echo "  test            Run all tests"
	@echo "  test-unit       Run unit tests only"
	@echo "  test-integration Run integration tests only"
	@echo "  test-coverage   Run tests with coverage"
	@echo ""
	@echo "$(BOLD)Database Targets:$(NORMAL)"
	@echo "  db-migrate      Run database migrations"
	@echo "  db-seed         Seed database with test data"
	@echo "  db-reset        Reset database (migrate + seed)"
	@echo "  db-generate     Generate sqlc code"
	@echo ""
	@echo "$(BOLD)Code Quality Targets:$(NORMAL)"
	@echo "  fmt             Format code with rustfmt"
	@echo "  fmt-check       Check code formatting"
	@echo "  clippy          Run clippy linter"
	@echo "  lint            Run all linters (fmt + clippy)"
	@echo "  audit           Check for security vulnerabilities"
	@echo "  outdated        Check for outdated dependencies"
	@echo ""
	@echo "$(BOLD)Docker Targets:$(NORMAL)"
	@echo "  docker-build    Build Docker image"
	@echo "  docker-up       Start Docker containers"
	@echo "  docker-down     Stop Docker containers"
	@echo "  docker-logs     View Docker logs"
	@echo "  docker-test     Run tests in Docker"
	@echo "  docker-publish  Build and push Docker image"
	@echo "  docker-shell    Open shell in Docker container"
	@echo ""
	@echo "$(BOLD)Clean Targets:$(NORMAL)"
	@echo "  clean           Clean build artifacts"
	@echo "  clean-all       Clean everything including Docker"
	@echo "  clean-cache     Clean cargo cache"
	@echo ""
	@echo "$(BOLD)Development Helpers:$(NORMAL)"
	@echo "  watch           Watch for changes and rebuild"
	@echo "  watch-test      Watch and run tests"
	@echo "  setup           Initial project setup"
	@echo "  doc             Generate documentation"
	@echo "  doc-open        Open documentation in browser"
	@echo "  deps            Check project dependencies"
	@echo "  help            Show this help message"
	@echo ""
	@echo "$(BOLD)================================================================================$(NORMAL)"
	@echo ""

# Default target
all: build
