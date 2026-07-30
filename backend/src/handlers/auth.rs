use crate::repository::sessions;
use crate::repository::users;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use bcrypt::{DEFAULT_COST, hash, verify};
use shared_lib::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) async fn register(
    State(pool): State<PgPool>,
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

    let existing = users::exists(&pool, username).await;

    match existing {
        Ok(true) => {
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

    let insert_result = users::create(&pool, username, &password_hash).await;

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

pub(crate) async fn login(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let username = payload.username.trim();
    let password = payload.password;

    let user_row = users::find_by_username(&pool, username).await;

    let (user_username, password_hash) = match user_row {
        Ok(Some((u, p))) => (u, p),
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    session_id: None,
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
                    session_id: None,
                    username: None,
                    message: format!("Database error: {}", e),
                }),
            );
        }
    };

    let is_valid = verify(&password, &password_hash).unwrap_or_default();

    if !is_valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(LoginResponse {
                success: false,
                token: None,
                session_id: None,
                username: None,
                message: "Invalid username or password.".to_string(),
            }),
        );
    }

    let token = Uuid::new_v4().to_string();
    let session_id = Uuid::new_v4().to_string();

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

    let session_result = sessions::create(
        &pool,
        &token,
        &session_id,
        &user_username,
        user_agent,
        ip_address,
    )
    .await;

    match session_result {
        Ok(_) => (
            StatusCode::OK,
            Json(LoginResponse {
                success: true,
                token: Some(token),
                session_id: Some(session_id),
                username: Some(user_username),
                message: "Logged in successfully.".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(LoginResponse {
                success: false,
                token: None,
                session_id: None,
                username: None,
                message: format!("Failed to create session: {}", e),
            }),
        ),
    }
}
