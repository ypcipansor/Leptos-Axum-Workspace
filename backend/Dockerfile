# Stage 1: Build the Rust backend binary
FROM rust:1.97-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace configuration and sources
COPY Cargo.toml Cargo.lock ./
COPY lib/ ./lib/
COPY backend/ ./backend/
COPY frontend/ ./frontend/

# Build backend binary
WORKDIR /app/backend
RUN cargo build --release

# Stage 2: Create the minimal execution image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (OpenSSL is needed for reqwest / PostgreSQL)
RUN apt-get update && apt-get install -y --no-install-recommends \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder stage
COPY --from=builder /app/target/release/backend /app/backend

# Expose backend port
EXPOSE 3000

ENV PORT=3000

CMD ["/app/backend"]
