use app_domain::AppState;
use tokio::task::JoinHandle;

/// Periodically delete expired session rows.
///
/// Expired sessions are already unusable -- every lookup filters on
/// `expires_at > now()` -- so this is housekeeping, not a security control.
/// Without it the table would grow forever, since the previous schema had no
/// expiry at all and nothing ever removed a row.
///
/// Returns the handle so the caller can stop the sweep during shutdown.
pub fn spawn_session_cleanup(state: AppState) -> JoinHandle<()> {
    let period = state.config.session_cleanup_interval;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(period);

        // The first tick fires immediately; skip it so a restart loop cannot
        // turn into a stream of DELETEs against a struggling database.
        ticker.tick().await;

        loop {
            ticker.tick().await;

            match state.auth.cleanup_expired_sessions().await {
                Ok(0) => tracing::debug!("session sweep found nothing to remove"),
                Ok(removed) => tracing::info!(removed, "expired sessions removed"),
                // A failed sweep is worth knowing about but must not end the
                // task -- the next tick will try again.
                Err(error) => tracing::warn!(%error, "session sweep failed"),
            }
        }
    })
}
