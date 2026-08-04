use app_domain::{Config, Environment};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Install the global tracing subscriber.
///
/// This replaces the seven scattered `eprintln!` calls the previous
/// implementation used as its entire logging story. Those produced unstructured
/// lines with no level, no timestamp, no request correlation and no way to
/// filter them in production.
///
/// Output format follows the environment: human-readable during development,
/// JSON in production so a log pipeline can index the fields rather than
/// regex over prose.
pub fn init(config: &Config) -> anyhow::Result<()> {
    // RUST_LOG wins when set. The default keeps the application at `info` while
    // silencing the per-query chatter from sqlx and the connection-level noise
    // from hyper, which would otherwise bury anything useful.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,hyper=warn,tower_http=info,h2=warn"));

    let registry = tracing_subscriber::registry().with(filter);

    match config.environment {
        Environment::Production => registry
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true)
                    .with_span_list(false),
            )
            .try_init(),
        Environment::Development => registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(false)
                    .compact(),
            )
            .try_init(),
    }
    .map_err(|e| anyhow::anyhow!("failed to install the tracing subscriber: {e}"))
}
