use tokio::signal;

/// Resolves when the process is asked to stop.
///
/// Handing this to `axum::serve(..).with_graceful_shutdown(..)` lets in-flight
/// requests finish instead of having their connections cut. The previous
/// implementation had no shutdown handling at all, so every deploy dropped
/// whatever was mid-flight.
pub async fn signal() {
    let ctrl_c = async {
        if let Err(error) = signal::ctrl_c().await {
            tracing::error!(%error, "failed to listen for Ctrl+C");
        }
    };

    // SIGTERM is what a container runtime actually sends; Ctrl+C alone would
    // leave `docker stop` and Kubernetes evictions to hit the kill timeout.
    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received Ctrl+C, shutting down"),
        () = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}
