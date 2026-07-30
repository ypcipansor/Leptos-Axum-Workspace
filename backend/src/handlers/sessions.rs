use crate::extractors::AuthenticatedUser;
use crate::repository::sessions;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use shared_lib::{RevokeRequest, RevokeResponse, SessionInfo};
use sqlx::{PgPool, Row};

pub(crate) async fn list_sessions(
    State(pool): State<PgPool>,
    auth_user: AuthenticatedUser,
) -> impl IntoResponse {
    let sessions_rows = sessions::list_by_username(&pool, &auth_user.username).await;

    match sessions_rows {
        Ok(rows) => {
            let list: Vec<SessionInfo> = rows
                .into_iter()
                .map(|r| {
                    let session_id: String = r.get("session_id");
                    let username: String = r.get("username");
                    let user_agent: Option<String> = r.get("user_agent");
                    let ip_address: Option<String> = r.get("ip_address");
                    let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");
                    SessionInfo {
                        is_current: session_id == auth_user.session_id,
                        id: session_id,
                        username,
                        user_agent,
                        ip_address,
                        created_at: created_at.to_rfc3339(),
                    }
                })
                .collect();

            (StatusCode::OK, Json(list))
        }
        Err(e) => {
            eprintln!("Database error during session listing: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(vec![]))
        }
    }
}

pub(crate) async fn revoke_session(
    State(pool): State<PgPool>,
    auth_user: AuthenticatedUser,
    Json(payload): Json<RevokeRequest>,
) -> impl IntoResponse {
    let rows_affected = sessions::delete(&pool, &payload.id, &auth_user.username).await;

    match rows_affected {
        Ok(count) => {
            if count > 0 {
                (
                    StatusCode::OK,
                    Json(RevokeResponse {
                        success: true,
                        message: "Session revoked successfully.".to_string(),
                    }),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(RevokeResponse {
                        success: false,
                        message: "Session not found or not owned by you.".to_string(),
                    }),
                )
            }
        }
        Err(e) => {
            eprintln!("Database error during session revocation: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RevokeResponse {
                    success: false,
                    message: "Internal server error.".to_string(),
                }),
            )
        }
    }
}
