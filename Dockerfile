# syntax=docker/dockerfile:1

# Builds the links-sig-server image. Usually built through docker/compose.yaml;
# see docker/README.md.

ARG RUST_VERSION=1.98
ARG DEBIAN_RELEASE=bookworm

# ---------------------------------------------------------------------------
# Build stage
# ---------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-${DEBIAN_RELEASE} AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libssl-dev \
        pkg-config \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY . .

# The cargo registry and target/ live in BuildKit cache mounts, so a source-only
# change reuses the compiled dependencies instead of rebuilding all of them.
# The binary has to be copied out because cache mounts are not part of the layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/links-sig-rust-server /usr/local/bin/links-sig-rust-server

# ---------------------------------------------------------------------------
# Runtime stage
# ---------------------------------------------------------------------------
FROM debian:${DEBIAN_RELEASE}-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --user-group --home-dir /app --no-create-home appuser

WORKDIR /app

COPY --from=builder /usr/local/bin/links-sig-rust-server /app/links-sig-rust-server
# The server serves ./static relative to its working directory.
COPY static /app/static

# The server always binds 0.0.0.0; SERVER_HOST only shows up in the startup log.
ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8081

EXPOSE 8081

USER appuser

HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=5 \
    CMD curl -fsS "http://127.0.0.1:${SERVER_PORT}/api/health" > /dev/null || exit 1

CMD ["/app/links-sig-rust-server"]
