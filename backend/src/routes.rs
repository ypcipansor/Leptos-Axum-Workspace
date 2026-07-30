use crate::handlers::auth::{login, register};
use crate::handlers::health::health;
use crate::handlers::sessions::{list_sessions, revoke_session};
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};

pub fn app(pool: PgPool) -> Router {
    let state = AppState { pool };

    let allowed_origin =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:1420".to_string());

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origin
                .parse::<axum::http::HeaderValue>()
                .expect("Invalid FRONTEND_ORIGIN"),
        )
        .allow_headers(Any)
        .allow_methods(Any);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/revoke", post(revoke_session))
        .layer(cors)
        .with_state(state)
}
