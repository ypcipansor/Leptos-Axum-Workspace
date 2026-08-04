# Task runner for the workspace. Install with: cargo install just
#
# Every command CI runs is defined here, so `just ci` locally is the same work
# the pipeline does -- no drift between the two.

set dotenv-load := true
set shell := ["bash", "-euo", "pipefail", "-c"]

database_url := env("DATABASE_URL", "postgres://postgres:postgres@127.0.0.1:5432/app")
test_database_url := env("TEST_DATABASE_URL", "postgres://postgres:postgres@127.0.0.1:5432/app_test")

# List available recipes.
default:
    @just --list --unsorted

# ---------------------------------------------------------------------------
# Development
# ---------------------------------------------------------------------------

# Run the app with hot reload on http://127.0.0.1:3000
dev:
    cargo leptos watch

# Serve a release build locally.
serve:
    cargo leptos serve --release

# Build the release artifacts into target/site.
build:
    cargo leptos build --release

# ---------------------------------------------------------------------------
# Database
# ---------------------------------------------------------------------------

# Apply all pending migrations.
migrate:
    sqlx migrate run --database-url "{{ database_url }}"

# Revert the most recent migration.
migrate-revert:
    sqlx migrate revert --database-url "{{ database_url }}"

# Create a new timestamped, reversible migration: just migrate-new add_widgets
migrate-new name:
    sqlx migrate add -r "{{ name }}"

# Drop, recreate and re-migrate the development database.
db-reset:
    sqlx database drop -y --database-url "{{ database_url }}"
    sqlx database create --database-url "{{ database_url }}"
    just migrate

# Refresh the committed .sqlx offline cache so `cargo check` works without a
# live database (this is what lets CI compile the query! macros).
prepare:
    cargo sqlx prepare --workspace --database-url "{{ database_url }}" -- --all-targets

# ---------------------------------------------------------------------------
# Quality gates
# ---------------------------------------------------------------------------

# Check formatting.
fmt-check:
    cargo fmt --all -- --check

# Apply formatting.
fmt:
    cargo fmt --all

# Lint every crate on both targets. The wasm pass matters: the old CI excluded
# the frontend crate entirely, so none of its code was ever linted.
lint:
    cargo clippy --workspace --all-targets --no-default-features --features ssr -- -D warnings
    cargo clippy -p frontend --target wasm32-unknown-unknown --no-default-features --features hydrate -- -D warnings

# Unit and integration tests.
test:
    SQLX_OFFLINE=true cargo test --workspace --no-default-features --features ssr

# Supply-chain audit: advisories, licences, banned crates, source pinning.
audit:
    cargo deny check

# Browser end-to-end tests.
e2e:
    cd end2end && npm ci && npx playwright test

# Everything CI runs, in the same order.
ci: fmt-check lint test audit build

# ---------------------------------------------------------------------------
# Housekeeping
# ---------------------------------------------------------------------------

# Remove build artifacts and generated assets.
clean:
    cargo clean
    rm -rf end2end/test-results end2end/playwright-report
