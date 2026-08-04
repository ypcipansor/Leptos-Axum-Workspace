//! Entrypoint.
//!
//! One process serves everything: server-rendered HTML, the hydration bundle,
//! the static assets and every `#[server]` endpoint. The previous deployment
//! needed two containers and an nginx in front of them to do less.

use std::net::SocketAddr;

use anyhow::Context;
use app_domain::{AppState, Config, db};
use leptos::config::get_configuration;
use server::{router, shutdown, tasks, telemetry};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Returns `Result` rather than panicking. Every step below reports what
    // went wrong and exits non-zero; the previous `main` called `.expect()` on
    // the configuration, the pool and the listener, so a missing environment
    // variable surfaced as a panic backtrace.
    let config = Config::from_env().context("invalid configuration")?;
    telemetry::init(&config)?;

    tracing::info!(
        environment = %config.environment,
        address = %config.bind_addr,
        "starting up"
    );

    let pool = db::connect(&config).context("failed to configure the database pool")?;

    // Migrations run before the listener opens, so an instance never serves
    // traffic against a schema it does not understand. sqlx takes an advisory
    // lock for the duration, so several replicas starting at once is safe.
    db::migrate(&pool)
        .await
        .context("failed to apply database migrations")?;
    tracing::info!("database migrations applied");

    let bind_addr = config.bind_addr;
    let state = AppState::new(pool, config);

    // Reads Cargo.toml's [[workspace.metadata.leptos]] during development, or
    // the LEPTOS_* environment variables in a release image where that file is
    // not present.
    let leptos_options = get_configuration(None)
        .context("failed to read the leptos configuration")?
        .leptos_options;

    let cleanup = tasks::spawn_session_cleanup(state.clone());
    let app = router::build(state, leptos_options);

    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;

    tracing::info!(address = %bind_addr, "listening");

    // `into_make_service_with_connect_info` is what puts the peer address into
    // request extensions. Without it the session audit trail would have no
    // address to record for a directly connected client.
    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown::signal())
    .await
    .context("server error");

    // Stopped only after in-flight requests have drained, so a request that
    // outlives the shutdown signal still sees a consistent world.
    cleanup.abort();
    tracing::info!("shutdown complete");

    result
}
