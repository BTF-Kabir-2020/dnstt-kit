# syntax=docker/dockerfile:1
# Multi-stage: build on rust, run on slim debian. Aimed at low-RAM hosts.

FROM rust:1.85-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY scanner-core ./scanner-core
COPY dns-cli ./dns-cli
# profiles / testdata not required to compile
RUN cargo build -p dns-cli --release && strip target/release/dns-cli || true

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd -m -u 10001 app
WORKDIR /work
COPY --from=builder /src/target/release/dns-cli /usr/local/bin/dns-cli
COPY testdata ./testdata
COPY config ./config
COPY .env.example ./.env.example
COPY docs ./docs
ENV DNS_CLI_BIND=0.0.0.0:8787
USER app
EXPOSE 8787
ENTRYPOINT ["dns-cli"]
CMD ["serve", "--bind", "0.0.0.0:8787"]
