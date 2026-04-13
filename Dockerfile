# Multi-stage build for both pulsar-api and pulsar-gateway.
# Build with: docker build --build-arg BINARY=pulsar-api -t pulsar-api .
#             docker build --build-arg BINARY=pulsar-gateway -t pulsar-gateway .

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ARG BINARY=pulsar-api
RUN cargo build --release -p $BINARY

FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
ARG BINARY=pulsar-api
COPY --from=builder /app/target/release/$BINARY /usr/local/bin/server
EXPOSE 3000
CMD ["/usr/local/bin/server"]
