# Stage 1: Build the static assets using Trunk
FROM rust:1.97-slim AS builder

WORKDIR /app

# Install build dependencies, Node.js (for Tailwind) and curl
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Add Wasm target
RUN rustup target add wasm32-unknown-unknown

# Download precompiled Trunk binary
RUN mkdir -p /usr/local/bin \
    && curl -fsSL https://github.com/trunk-rs/trunk/releases/download/v0.21.1/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf - -C /usr/local/bin

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY lib/ ./lib/
COPY frontend/ ./frontend/

WORKDIR /app/frontend

# Compile Tailwind CSS
RUN npx tailwindcss@3.4.15 -c tailwind.config.js -i input.css -o style.css

# Build frontend static files
RUN trunk build --release

# Stage 2: Serve the static files with Nginx
FROM nginx:alpine

# Copy static assets from builder stage
COPY --from=builder /app/frontend/dist /usr/share/nginx/html

# Copy Nginx proxy configuration
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
