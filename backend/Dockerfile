# Build stage
FROM rust:1.94-slim AS builder
WORKDIR /app
ARG CARGO_FEATURES="default"
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev libcurl4-openssl-dev curl protobuf-compiler \
    build-essential cmake \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto ./proto
COPY benches ./benches
# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --no-default-features --features "${CARGO_FEATURES}" 2>/dev/null || true
# Remove dummy build artifacts so real source gets compiled
RUN rm -rf src target/release/deps/rust_conductor* target/release/rust-conductor*
# Copy actual source
COPY src ./src
RUN cargo build --release --no-default-features --features "${CARGO_FEATURES}"

# Runtime stage
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN groupadd -r appuser && useradd -r -g appuser appuser
COPY --from=builder /app/target/release/rust-conductor /usr/local/bin/rust-conductor
USER appuser
EXPOSE 8090 50055
CMD ["rust-conductor"]
