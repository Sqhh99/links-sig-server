FROM rust:1.92-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl-dev \
        pkg-config \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system appuser \
    && useradd --system --gid appuser --create-home --home-dir /app appuser

COPY --from=builder /app/target/release/links-sig-rust-server /app/links-sig-rust-server
COPY --from=builder /app/static /app/static

EXPOSE 8081

USER appuser

CMD ["/app/links-sig-rust-server"]
