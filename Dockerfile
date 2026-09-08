FROM rust:1-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --bin invoice-service --bin mock-psp --bin create-api-key

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/invoice-service /usr/local/bin/invoice-service
COPY --from=builder /app/target/release/mock-psp /usr/local/bin/mock-psp
COPY --from=builder /app/target/release/create-api-key /usr/local/bin/create-api-key

EXPOSE 8080 9090
