# 0k-sync relay server
# Multi-stage build: compile in Rust image, run in minimal Debian

# ---- Builder ----
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build -p zerok-sync-relay --release

# ---- Runtime ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system relay && useradd --system --gid relay --home-dir /data --shell /usr/sbin/nologin relay
RUN mkdir -p /data /etc/0k-sync && chown relay:relay /data

COPY --from=builder /app/target/release/zerok-sync-relay /usr/local/bin/zerok-sync-relay
COPY --from=builder /app/sync-relay/relay.docker.toml /etc/0k-sync/relay.toml

USER relay
VOLUME /data
EXPOSE 8080

ENV RUST_LOG=sync_relay=info,iroh=warn

# Binary handles SIGINT (tokio::signal::ctrl_c), not SIGTERM
STOPSIGNAL SIGINT

HEALTHCHECK --interval=10s --timeout=3s --retries=3 --start-period=5s \
    CMD curl -sf http://localhost:8080/health || exit 1

CMD ["zerok-sync-relay", "--config", "/etc/0k-sync/relay.toml"]
