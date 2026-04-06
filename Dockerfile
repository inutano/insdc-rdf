# Build stage
FROM rust:1-slim AS builder
WORKDIR /build
ARG BUILD_VERSION=0.0.0
ENV BUILD_VERSION=$BUILD_VERSION
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY src/ src/
COPY crates/ crates/
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/insdc-rdf /usr/local/bin/insdc-rdf
WORKDIR /data
ENTRYPOINT ["insdc-rdf"]
