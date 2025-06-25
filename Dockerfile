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

# Create a non-root user
RUN useradd -r -s /bin/false appuser

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/target/release/audio-server /usr/local/bin/audio-server

# Change ownership to non-root user
RUN chown appuser:appuser /usr/local/bin/audio-server

# Switch to non-root user
USER appuser

# Expose port (adjust as needed)
EXPOSE 8080

# Run the binary
ENTRYPOINT ["audio-server"]