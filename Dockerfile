# syntax=docker/dockerfile:1
# Multi-stage: build on rust, run on slim debian. Aimed at ~512MB hosts.
# Web UI assets are compiled into dns-cli (include_str); no Node in image.
# slipnet is NOT bundled — mount vendor/ or use --skip-slipnet.

FROM rust:1.85-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY scanner-core ./scanner-core
COPY dns-cli ./dns-cli
RUN cargo build -p dns-cli --release \
  && strip target/release/dns-cli || true

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd -m -u 10001 app
WORKDIR /work
COPY --from=builder /src/target/release/dns-cli /usr/local/bin/dns-cli
COPY testdata ./testdata
COPY config ./config
COPY docs ./docs
COPY .env.example ./.env.example
ENV DNS_CLI_BIND=0.0.0.0:8787 \
    DNS_CLI_WORK_DIR=/work
# Default image user is non-root. Compose overrides to root when bind-mounting ./ 
# so runs/backups stay writable on the host.
USER app
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD dns-cli info >/dev/null 2>&1 || exit 1
ENTRYPOINT ["dns-cli"]
CMD ["serve", "--bind", "0.0.0.0:8787"]
