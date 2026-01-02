# Multi-stage build for Rust axum application
# Stage 1: Builder - Compile the application
FROM mirror.gcr.io/library/rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Download and cache dependencies
RUN cargo fetch && cargo build --release --locked

# Copy source code
COPY src/ ./src/
COPY configs/ ./configs/
COPY sqlc/ ./sqlc/

# Build the application with optimization flags
ARG BUILD_VERSION=dev
ARG BUILD_TIME
RUN cargo build --release \
    --bin zercle-rust-template \
    --locked \
    --no-default-features \
    --features ""

# Final stage - distroless minimal runtime
FROM gcr.io/distroless/base AS runner

ENV TZ=Asia/Bangkok
ENV LANG=C.UTF-8

WORKDIR /opt/app

# Copy tini static binary from builder
COPY --from=builder /usr/bin/tini-static /usr/bin/tini

# Copy binary from builder
COPY --from=builder /app/target/release ./

# Copy config files
COPY --from=builder /app/configs ./configs

# Expose port
EXPOSE 3000

# Run the application with tini for proper signal handling
ENTRYPOINT ["/usr/bin/tini", "--"]

CMD ["zercle-rust-template"]
