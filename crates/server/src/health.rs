use app_core::{HealthStatus, ServiceState};
use app_domain::{AppState, SERVICE_NAME, SERVICE_VERSION, db};
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use http::StatusCode;

/// Liveness: is the process running and able to answer?
///
/// Never touches the database. An orchestrator reads this to decide whether to
/// *restart* the container, and restarting a healthy process because its
/// database is briefly unreachable turns a recoverable blip into an outage.
pub async fn live() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// Readiness: should this instance receive traffic?
///
/// Checks the database, because an instance that cannot reach it can serve
/// nothing useful. A load balancer reads this to decide whether to *route* to
/// the instance -- a distinction the previous single `/api/health` endpoint,
/// which reported a hard-coded "ok", could not express at all.
pub async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let database = match db::ping(state.pool()).await {
        Ok(()) => ServiceState::Up,
        Err(error) => {
            tracing::warn!(%error, "readiness probe failed");
            ServiceState::Down
        }
    };

    let status = HealthStatus::new(SERVICE_NAME, SERVICE_VERSION, database);

    // The status code carries the verdict so a probe does not have to parse the
    // body, while the body explains which dependency is at fault.
    let code = if status.is_ready() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (code, Json(status))
}
