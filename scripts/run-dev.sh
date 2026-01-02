#!/bin/bash
set -e

# Run Rust development server
# Usage: ./scripts/run-dev.sh [environment]

ENVIRONMENT=${1:-local}
CONFIG_FILE="configs/${ENVIRONMENT}.yaml"

echo "=========================================="
echo "Starting Zercle Rust Template Development Server"
echo "=========================================="
echo "Environment: ${ENVIRONMENT}"
echo ""

# Set development environment variables
export SERVER_ENV=${ENVIRONMENT}
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Check if config file exists
if [ -f "${CONFIG_FILE}" ]; then
    echo "Using config file: ${CONFIG_FILE}"
fi

# Check for PostgreSQL
echo "Checking database connection..."
if command -v pg_isready &> /dev/null; then
    DB_HOST=${DB_HOST:-localhost}
    DB_PORT=${DB_PORT:-5432}
    if pg_isready -h ${DB_HOST} -p ${DB_PORT} > /dev/null 2>&1; then
        echo "✅ Database is ready"
    else
        echo "⚠️  Database is not ready. Make sure PostgreSQL is running."
        echo "   You can start it with: make docker-up"
    fi
else
    echo "⚠️  psql not found, skipping database check"
fi

echo ""
echo "Starting development server..."
echo "Press Ctrl+C to stop"
echo ""

# Build and run with hot reload using cargo watch if available
if command -v cargo-watch &> /dev/null; then
    echo "Using cargo-watch for hot reload..."
    cargo watch -x run --bin zercle-rust-template
else
    echo "Installing cargo-watch for hot reload..."
    cargo install cargo-watch
    cargo watch -x run --bin zercle-rust-template
fi
