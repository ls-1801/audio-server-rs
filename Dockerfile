# Multi-stage build for Rust application
FROM rust:1.85 as builder

# Create app directory
WORKDIR /usr/src/app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/target/release/audio-server /usr/local/bin/audio-server

# Expose port (adjust as needed)
EXPOSE 8080

# Run the binary
ENTRYPOINT ["audio-server"]