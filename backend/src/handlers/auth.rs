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

    // Validate username lengths
    if username.is_empty() || username.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: "Username must be between 1 and 100 characters.".to_string(),
            }),
        );
    }

    // Validate password length for bcrypt safety (limit to 72 bytes)
    if password.len() < 4 || password.len() > 72 {
        return (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: "Password must be between 4 and 72 characters.".to_string(),
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
            eprintln!("Database error during registration check: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegisterResponse {
                    success: false,
                    message: "Internal server error.".to_string(),
                }),
            );
        }
        _ => {}
    }

    // Move CPU-heavy hashing to spawn_blocking
    let password_clone = password.clone();
    let hashed = tokio::task::spawn_blocking(move || hash(&password_clone, DEFAULT_COST)).await;

    let password_hash = match hashed {
        Ok(Ok(h)) => h,
        _ => {
            eprintln!("Failed to hash password during registration");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegisterResponse {
                    success: false,
                    message: "Internal server error.".to_string(),
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
            eprintln!("Database error during registration insert: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RegisterResponse {
                    success: false,
                    message: "Internal server error.".to_string(),
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
            // Mitigate timing attack by executing dummy bcrypt verify
            let dummy_hash =
                "$2b$12$6uX7/IAt32VzE/tq1.D68OhZpfe6t.v6TjGg1r9FWeM20xV9y7G1e".to_string();
            let password_clone = password.clone();
            let _ = tokio::task::spawn_blocking(move || verify(&password_clone, &dummy_hash)).await;
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
            eprintln!("Database error during login user lookup: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    session_id: None,
                    username: None,
                    message: "Internal server error.".to_string(),
                }),
            );
        }
    };

    // Move CPU-heavy password verification to spawn_blocking
    let password_clone = password.clone();
    let password_hash_clone = password_hash.clone();
    let verified =
        tokio::task::spawn_blocking(move || verify(&password_clone, &password_hash_clone)).await;

    let is_valid = match verified {
        Ok(Ok(valid)) => valid,
        _ => false,
    };

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
        Err(e) => {
            eprintln!("Database error during login session creation: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LoginResponse {
                    success: false,
                    token: None,
                    session_id: None,
                    username: None,
                    message: "Internal server error.".to_string(),
                }),
            )
        }
    }
}
