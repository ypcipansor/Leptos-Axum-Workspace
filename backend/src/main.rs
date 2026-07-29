use axum::{Json, Router, routing::get};
use shared_lib::HealthStatus;
use tokio::net::TcpListener;

const SERVICE_NAME: &str = "backend";

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/health", get(health));

    let listener = TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind backend server to 0.0.0.0:3000");

    println!(
        "backend listening on http://{}",
        listener.local_addr().expect("listener address unavailable")
    );

    axum::serve(listener, app)
        .await
        .expect("backend server failed");
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
