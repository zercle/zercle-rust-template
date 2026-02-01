#!/bin/bash
set -e

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Set defaults
export RUST_LOG=${RUST_LOG:-debug}
export SERVER_ENV=${SERVER_ENV:-local}

echo "=========================================="
echo "Starting Zercle Rust Template Development Server"
echo "=========================================="
echo "Environment: $SERVER_ENV"
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
