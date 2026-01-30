FROM rust:1.83 AS builder

WORKDIR /app

# Copy source files
COPY . .

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libc6 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/blazing_SEARCH /usr/local/bin/blazing_SEARCH

# Copy the web directory and other necessary files
COPY --from=builder /app/web /app/web
COPY --from=builder /app/config.toml /app/config.toml
COPY --from=builder /app/config.example.toml /app/config.example.toml
COPY --from=builder /app/README_LINUX.md /app/README_LINUX.md

# Create cache directory
RUN mkdir -p /app/nakazi_cache

EXPOSE 8080

CMD ["blazing_SEARCH", "web"]