# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Builder — musl static build for distroless/static final image.
#
# Base image: `messense/rust-musl-cross:x86_64-musl` — Alpine + a pinned
# `x86_64-unknown-linux-musl` Rust toolchain + a working musl C compiler.
# Chosen over `rust:1-bookworm` because the Debian `musl-tools` package on
# that image ships a `musl-gcc` whose cc1 rejects the `-m64` flag that the
# `cc` 1.2.x build-helper auto-prepends for 64-bit musl targets, breaking
# `ring`/`aws-lc-rs`/etc. compilation. The messense image is the
# well-known production-ready alternative.
#
# The crate stack is fully rust-native (sqlx runtime-tokio-rustls, redis
# tokio-rustls-comp), so we link against musl libc instead of glibc and
# produce a truly static binary that mirrors the Go template's
# `CGO_ENABLED=0` static build and lets the final image be
# `gcr.io/distroless/static-debian12:nonroot` (no glibc, no shell).
# -----------------------------------------------------------------------------
FROM messense/rust-musl-cross:x86_64-musl AS builder

WORKDIR /build

# build.rs invokes protoc via tonic-build. Install protobuf-compiler.
USER root
RUN apt-get update \
 && apt-get install -y --no-install-recommends protobuf-compiler \
 && rm -rf /var/lib/apt/lists/*

ENV CC_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-gcc \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-unknown-linux-musl-gcc \
    RUSTFLAGS="-C link-arg=-static -C target-feature=+crt-static"

# ---- dependency cache layer ----
# Copy manifests first so a source-only change does not bust the cargo
# registry/cache layer. build.rs is included so tonic-build changes also
# invalidate the cache correctly.
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY proto ./proto
COPY migrations ./migrations
COPY config.yaml ./config.yaml

# Build metadata injected at compile time so `option_env!("VERSION")` etc.
# in src/app.rs pick them up (defaults match Go template).
ARG VERSION=dev
ARG COMMIT_SHA=unknown
ARG BUILD_TIME=unknown

ENV VERSION=${VERSION} \
    COMMIT_SHA=${COMMIT_SHA} \
    BUILD_TIME=${BUILD_TIME}

RUN cargo build --release \
    --target x86_64-unknown-linux-musl \
    --bin server

# -----------------------------------------------------------------------------
# Final — distroless/static, nonroot user. Truly static binary, no shell,
# no package manager. Matches the Go template's runtime surface.
# -----------------------------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder --chown=nonroot:nonroot \
    /build/target/x86_64-unknown-linux-musl/release/server /server

# Default config shipped with the image. compose.yml overrides via volume
# mounts in practice; this is the fallback baked into the image.
COPY --from=builder --chown=nonroot:nonroot /build/config.yaml /config.yaml

USER nonroot:nonroot

EXPOSE 8080
EXPOSE 50051

ENTRYPOINT ["/server"]
