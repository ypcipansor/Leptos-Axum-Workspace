use std::sync::Arc;

use sqlx::PgPool;

use crate::auth::AuthService;
use crate::config::Config;

/// Shared application state.
///
/// Cloning is cheap: the pool is internally reference counted and the config is
/// behind an `Arc`. Axum clones state per request, and Leptos server functions
/// receive a clone through `provide_context`, so this must stay cheap to copy.
#[derive(Debug, Clone)]
pub struct AppState {
    pub auth: AuthService,
    pub config: Arc<Config>,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self {
            auth: AuthService::new(pool, config.session_ttl),
            config: Arc::new(config),
        }
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        self.auth.pool()
    }
}
