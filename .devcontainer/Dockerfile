# Multi-stage build for Kotoba (local development and GKE)
FROM rust:1.82-slim AS builder

# Install required packages
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    clang \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build the application with limited parallel jobs for low memory systems
RUN cargo build --release --features full --jobs 2

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/bash kotoba

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/kotoba /app/kotoba

# Copy configuration files
COPY examples/deploy/ /app/config/

# Create data directory
RUN mkdir -p /app/data && chown kotoba:kotoba /app/data

# Switch to non-root user
USER kotoba

# Expose ports
EXPOSE 3000 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Default command
CMD ["./kotoba", "server", "--config", "/app/config/simple.kotoba-deploy"]
