# 1. Build stage
FROM rust:latest AS builder
WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() { println!(\"hello\"); }" > src/main.rs
RUN cargo fetch

# Copy your code and build in release mode
COPY . .
RUN cargo build --release

# 2. Runtime stage
FROM debian:12-slim
RUN apt-get update \
 && apt-get install -y ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Copy the built binary
COPY --from=builder /app/target/release/docfusiondb /usr/local/bin/docfusiondb

# Default entrypoint
ENTRYPOINT ["docfusiondb"]
