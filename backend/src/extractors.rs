use crate::repository::sessions::lookup_by_token;
use axum::{Json, extract::FromRef, http::StatusCode};
use sqlx::PgPool;

pub struct AuthenticatedUser {
    pub username: String,
    pub token: String,
    pub session_id: String,
}

impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    PgPool: FromRef<S>,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);

        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok());

        let token = match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                header["Bearer ".len()..].trim().to_string()
            }
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "Missing or invalid authorization token" })),
                ));
            }
        };

        let session = lookup_by_token(&pool, &token).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Database error" })),
            )
        })?;

        match session {
            Some((username, session_id)) => Ok(AuthenticatedUser {
                username,
                token,
                session_id,
            }),
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Session is invalid or has been revoked" })),
            )),
        }
    }
}
