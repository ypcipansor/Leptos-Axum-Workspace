use axum::{
    Json, Router,
    extract::{FromRef, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use shared_lib::{
    HealthStatus, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, SessionInfo,
};
use sqlx::{PgPool, Row};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

const SERVICE_NAME: &str = "backend";

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

pub struct AuthenticatedUser {
    pub username: String,
    pub token: String,
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

        let session = sqlx::query("SELECT username FROM sessions WHERE token = $1")
            .bind(&token)
            .fetch_optional(&pool)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Database error" })),
                )
            })?;

        match session {
            Some(row) => {
                let username: String = row.get("username");
                Ok(AuthenticatedUser { username, token })
            }
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Session is invalid or has been revoked" })),
            )),
        }
    }
}

pub async fn init_db(database_url: &str) -> PgPool {
    let pool = PgPool::connect(database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(100) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    )
    .execute(&pool)
    .await
    .expect("Failed to create users table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            token VARCHAR(255) PRIMARY KEY,
            username VARCHAR(100) NOT NULL REFERENCES users(username) ON DELETE CASCADE,
            user_agent TEXT,
            ip_address VARCHAR(45),
            created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    )
    .execute(&pool)
    .await
    .expect("Failed to create sessions table");

    pool
}

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

async fn health() -> Json<HealthStatus> {
    Json(HealthStatus::ok(SERVICE_NAME))
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let username = payload.username.trim();
    let password = payload.password;

    if username.is_empty() || password.len() < 4 {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: "Username cannot be empty and password must be at least 4 characters."
                    .to_string(),
            }),
        );
    }

    let existing = sqlx::query("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.pool)
        .await;

    match existing {
        Ok(Some(_)) => {
            return (
                StatusCode::CONFLICT,
                Json(RegisterResponse {
                    success: false,
                    message: "Username already exists.".to_string(),
                }),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegisterResponse {
                    success: false,
                    message: format!("Database error: {}", e),
                }),
            );
        }
        _ => {}
    }

    let password_hash = match hash(&password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegisterResponse {
                    success: false,
                    message: "Failed to hash password.".to_string(),
                }),
            );
        }
    };

    let insert_result = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2)")
        .bind(username)
        .bind(password_hash)
        .execute(&state.pool)
        .await;

    match insert_result {
        Ok(_) => (
            StatusCode::CREATED,
            Json(RegisterResponse {
                success: true,
                message: "User registered successfully.".to_string(),
            }),
        ),
        Err(e) => {
            if matches!(&e, sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505"))
            {
                return (
                    StatusCode::CONFLICT,
                    Json(RegisterResponse {
                        success: false,
                        message: "Username already exists.".to_string(),
                    }),
                );
            }
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegisterResponse {
                    success: false,
                    message: format!("Failed to save user: {}", e),
                }),
            )
        }
    }
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let username = payload.username.trim();
    let password = payload.password;

    let user_row = sqlx::query("SELECT username, password_hash FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.pool)
        .await;

    let user_row = match user_row {
        Ok(Some(row)) => row,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    username: None,
                    message: "Invalid username or password.".to_string(),
                }),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    username: None,
                    message: format!("Database error: {}", e),
                }),
            );
        }
    };

    let password_hash: String = user_row.get("password_hash");
    let user_username: String = user_row.get("username");

    let is_valid = verify(&password, &password_hash).unwrap_or_default();

    if !is_valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                username: None,
                message: "Invalid username or password.".to_string(),
            }),
        );
    }

    let token = Uuid::new_v4().to_string();

    let user_agent = headers
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let ip_address = headers
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() <= 45)
        .or_else(|| Some("127.0.0.1".to_string()));

    let session_result = sqlx::query(
        "INSERT INTO sessions (token, username, user_agent, ip_address) VALUES ($1, $2, $3, $4)",
    )
    .bind(&token)
    .bind(&user_username)
    .bind(user_agent)
    .bind(ip_address)
    .execute(&state.pool)
    .await;

    match session_result {
        Ok(_) => (
            StatusCode::OK,
            Json(LoginResponse {
                success: true,
                token: Some(token),
                username: Some(user_username),
                message: "Logged in successfully.".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LoginResponse {
                success: false,
                token: None,
                username: None,
                message: format!("Failed to create session: {}", e),
            }),
        ),
    }
}

async fn list_sessions(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
) -> impl IntoResponse {
    let sessions_rows = sqlx::query(
        "SELECT token, username, user_agent, ip_address, created_at FROM sessions WHERE username = $1 ORDER BY created_at DESC",
    )
    .bind(&auth_user.username)
    .fetch_all(&state.pool)
    .await;

    match sessions_rows {
        Ok(rows) => {
            let list: Vec<SessionInfo> = rows
                .into_iter()
                .map(|r| {
                    let token: String = r.get("token");
                    let username: String = r.get("username");
                    let user_agent: Option<String> = r.get("user_agent");
                    let ip_address: Option<String> = r.get("ip_address");
                    let created_at: chrono::DateTime<chrono::Utc> = r.get("created_at");
                    SessionInfo {
                        is_current: token == auth_user.token,
                        token,
                        username,
                        user_agent,
                        ip_address,
                        created_at: created_at.to_rfc3339(),
                    }
                })
                .collect();

            (StatusCode::OK, Json(list))
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(vec![])),
    }
}

async fn revoke_session(
    State(state): State<AppState>,
    auth_user: AuthenticatedUser,
    Json(payload): Json<shared_lib::RevokeRequest>,
) -> impl IntoResponse {
    let delete_result = sqlx::query("DELETE FROM sessions WHERE token = $1 AND username = $2")
        .bind(&payload.token)
        .bind(&auth_user.username)
        .execute(&state.pool)
        .await;

    match delete_result {
        Ok(res) => {
            if res.rows_affected() > 0 {
                (
                    StatusCode::OK,
                    Json(shared_lib::RevokeResponse {
                        success: true,
                        message: "Session revoked successfully.".to_string(),
                    }),
                )
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(shared_lib::RevokeResponse {
                        success: false,
                        message: "Session not found or not owned by you.".to_string(),
                    }),
                )
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(shared_lib::RevokeResponse {
                success: false,
                message: format!("Database error: {}", e),
            }),
        ),
    }
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
