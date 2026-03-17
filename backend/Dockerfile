# Build stage
FROM rust:1.94-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev curl protobuf-compiler \
    cmake build-essential \
    zlib1g-dev libsasl2-dev \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto ./proto
# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
# Remove dummy build artifacts so real source gets compiled
RUN rm -rf src target/release/deps/rust_conductor* target/release/rust-conductor*
# Copy actual source
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rust-conductor /usr/local/bin/rust-conductor
EXPOSE 8080 50051
CMD ["rust-conductor"]
