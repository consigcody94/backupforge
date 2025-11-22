# Build stage
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.toml
COPY crates crates

# Build dependencies (cached layer)
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

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
