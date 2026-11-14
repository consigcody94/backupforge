# Build stage
FROM rust:1.99-slim@sha256:f6c4a35d21dbb3c89d7a8d056c193e94fc09037f2b58dd858b09c39a47dd02d7 AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates crates

# Build dependencies (cached layer)
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 backupforge

# Copy binary from builder
COPY --from=builder /app/target/release/backupforge /usr/local/bin/backupforge

# Create directories
RUN mkdir -p /data /config && \
    chown -R backupforge:backupforge /data /config

# Switch to app user
USER backupforge

# Volumes
VOLUME ["/data", "/config"]

# Expose API port
EXPOSE 8080

# Set entrypoint
ENTRYPOINT ["backupforge"]

# Default command
CMD ["server", "--config", "/config/config.toml"]
