# syntax=docker/dockerfile:1.7

# One image for the whole application.
#
# The previous setup built two: a backend image that copied `frontend/` and a
# frontend image that copied `backend/`, so a change to either invalidated the
# cache in both, and neither had a .dockerignore -- every build uploaded the
# entire `target/` directory as context.

ARG RUST_VERSION=1.97.1
ARG CARGO_LEPTOS_VERSION=0.3.7
ARG DEBIAN_RELEASE=bookworm

# ---------------------------------------------------------------------------
# Base toolchain
# ---------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-slim-${DEBIAN_RELEASE} AS chef

# No openssl here: sqlx and reqwest are configured with rustls, so the TLS
# stack is compiled in. The previous images installed libssl-dev for nothing.
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config \
 && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

ARG CARGO_LEPTOS_VERSION
ADD https://github.com/leptos-rs/cargo-leptos/releases/download/v${CARGO_LEPTOS_VERSION}/cargo-leptos-x86_64-unknown-linux-gnu.tar.gz /tmp/cargo-leptos.tar.gz
RUN tar -xzf /tmp/cargo-leptos.tar.gz -C /tmp \
 && install -m0755 /tmp/cargo-leptos-x86_64-unknown-linux-gnu/cargo-leptos /usr/local/bin/cargo-leptos \
 && rm -rf /tmp/cargo-leptos*

RUN cargo install cargo-chef --locked --version ^0.1

WORKDIR /build

# ---------------------------------------------------------------------------
# Dependency plan
# ---------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
FROM chef AS builder

# Dependencies are compiled from the recipe alone, so editing application
# source does not rebuild them. This is the difference between a ten-second
# rebuild and a ten-minute one.
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo chef cook --release --target wasm32-unknown-unknown --recipe-path recipe.json

COPY . .

# Compiles the sqlx query macros against the committed .sqlx cache instead of
# reaching for a database that does not exist inside the build.
ENV SQLX_OFFLINE=true

# Builds the server binary, the wasm bundle and the stylesheet in one step.
# cargo-leptos fetches the standalone Tailwind binary itself -- there is no
# Node.js in this image and no `npx tailwindcss@...` at build time.
RUN cargo leptos build --release

# ---------------------------------------------------------------------------
# Runtime
# ---------------------------------------------------------------------------
FROM debian:${DEBIAN_RELEASE}-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# A dedicated unprivileged account. The previous images ran as root, so a
# process compromise started with full control of the container.
RUN groupadd --system --gid 10001 app \
 && useradd --system --uid 10001 --gid app --no-create-home app

WORKDIR /app

COPY --from=builder --chown=app:app /build/target/release/server ./server
COPY --from=builder --chown=app:app /build/target/site ./site

USER app

ENV APP_ENV=production \
    HOST=0.0.0.0 \
    PORT=3000 \
    LEPTOS_SITE_ROOT=/app/site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_OUTPUT_NAME=app \
    LEPTOS_SITE_ADDR=0.0.0.0:3000

EXPOSE 3000

# Readiness, not liveness: this reports unhealthy when the database is
# unreachable, which is what an orchestrator needs to stop routing traffic here.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl --fail --silent --show-error http://127.0.0.1:3000/health/ready || exit 1

CMD ["./server"]
