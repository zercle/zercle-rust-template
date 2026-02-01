# Stage 1: Builder
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY cmd ./cmd
COPY migrations ./migrations
COPY configs ./configs

# Build release binary
RUN cargo build --release --bin zercle-rust-template

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Copy binary from builder
COPY --from=builder /app/target/release/zercle-rust-template ./server

# Copy configs and migrations
COPY --from=builder /app/configs ./configs
COPY --from=builder /app/migrations ./migrations

# Set permissions
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the binary
CMD ["./server"]
