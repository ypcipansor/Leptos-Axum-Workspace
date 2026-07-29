use axum::{Json, Router, routing::get};
use shared_lib::HealthStatus;

const SERVICE_NAME: &str = "backend";

pub fn app() -> Router {
    Router::new().route("/api/health", get(health))
}

async fn health() -> Json<HealthStatus> {
    Json(HealthStatus::ok(SERVICE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_returns_ok_status() {
        let Json(body) = health().await;

        assert_eq!(body, HealthStatus::ok(SERVICE_NAME));
    }
}
