# syntax=docker/dockerfile:1
FROM rust:1.99-bookworm AS builder
WORKDIR /workspace/api
COPY api/Cargo.toml api/Cargo.lock ./
COPY api/src ./src
RUN cargo build --release --locked --bin arcadia-api

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/api/target/release/arcadia-api /usr/local/bin/arcadia-api
ENV PORT=8080
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/arcadia-api"]
