# syntax=docker/dockerfile:1

# -----------------------------------------------------------------------------
# Builder — musl static build for distroless/static final image.
#
# Base image: `messense/rust-musl-cross:${RUST_MUSL_TAG}` (default
# `x86_64-musl`) — Alpine + a pinned musl Rust toolchain + a working musl C
# compiler. Chosen over `rust:1-bookworm` because the Debian `musl-tools`
# package on that image ships a `musl-gcc` whose cc1 rejects the `-m64` flag
# that the `cc` 1.2.x build-helper auto-prepends for 64-bit musl targets,
# breaking `ring`/`aws-lc-rs`/etc. compilation. The messense image is the
# well-known production-ready alternative.
#
# All three arch-specific values (base-image tag, Cargo target triple,
# musl-gcc binary name) are exposed as ARGs with x86_64 defaults, so the
# build can be retargeted by passing, e.g. for aarch64:
#   docker build --build-arg RUST_MUSL_TAG=aarch64-musl \
#                --build-arg MUSL_TARGET=aarch64-unknown-linux-musl \
#                --build-arg MUSL_GCC=aarch64-unknown-linux-musl-gcc \
#                -f Containerfile -t app .
#
# The crate stack is fully rust-native (sqlx runtime-tokio-rustls, redis
# tokio-rustls-comp), so we link against musl libc instead of glibc and
# produce a truly static binary that mirrors the Go template's
# `CGO_ENABLED=0` static build and lets the final image be
# `gcr.io/distroless/static-debian13:nonroot` (no glibc, no shell).
# -----------------------------------------------------------------------------
ARG RUST_MUSL_TAG=x86_64-musl
ARG MUSL_TARGET=x86_64-unknown-linux-musl
ARG MUSL_GCC=x86_64-unknown-linux-musl-gcc
# Cargo/BuildKit translate the target triple into env-var names with
# underscores. `MUSL_TARGET_UNDERSCORE` is the lowercase form (used by
# cargo's `CC_<triple>` env var), and `MUSL_TARGET_UPPER` is the
# uppercase form (required by cargo's `CARGO_TARGET_<triple>_LINKER`).
# Both are passed in as ARGs so the env-var names below are derived,
# not hardcoded.
ARG MUSL_TARGET_UNDERSCORE=x86_64_unknown_linux_musl
# Cargo's CARGO_TARGET_<triple>_LINKER requires the triple UPPERCASE
# (CC_<triple> uses lowercase). Keep both forms as ARGs.
ARG MUSL_TARGET_UPPER=X86_64_UNKNOWN_LINUX_MUSL

FROM messense/rust-musl-cross:${RUST_MUSL_TAG} AS builder

WORKDIR /build

# build.rs invokes protoc via tonic-build. Install protobuf-compiler.
USER root
RUN apt-get update \
 && apt-get install -y --no-install-recommends protobuf-compiler \
 && rm -rf /var/lib/apt/lists/*

ARG MUSL_TARGET
ARG MUSL_GCC
ARG MUSL_TARGET_UNDERSCORE
ARG MUSL_TARGET_UPPER
ENV CC_${MUSL_TARGET_UNDERSCORE}=${MUSL_GCC} \
    CARGO_TARGET_${MUSL_TARGET_UPPER}_LINKER=${MUSL_GCC} \
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
    --target ${MUSL_TARGET} \
    --bin server

# -----------------------------------------------------------------------------
# Final — distroless/static, nonroot user. Truly static binary, no shell,
# no package manager. Matches the Go template's runtime surface.
# -----------------------------------------------------------------------------
FROM gcr.io/distroless/static-debian13:nonroot

ARG MUSL_TARGET

COPY --from=builder --chown=nonroot:nonroot \
    /build/target/${MUSL_TARGET}/release/server /server

# Default config shipped with the image. compose.yml overrides via volume
# mounts in practice; this is the fallback baked into the image.
COPY --from=builder --chown=nonroot:nonroot /build/config.yaml /config.yaml

USER nonroot:nonroot

EXPOSE 8080
EXPOSE 50051

ENTRYPOINT ["/server"]
