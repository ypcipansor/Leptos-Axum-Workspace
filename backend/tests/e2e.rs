use backend::{app, init_db};
use shared_lib::{
    HealthStatus, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, RevokeResponse,
    SessionInfo,
};
use std::env;
use tokio::net::TcpListener;

async fn setup_test_app() -> (String, tokio::task::JoinHandle<()>, sqlx::PgPool) {
    let database_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://postgres:postgres@localhost:5432/postgres_test".to_string()
    });

    let pool = init_db(&database_url).await;

    // Clean up tables to make tests fully reproducible and independent
    sqlx::query("TRUNCATE TABLE sessions CASCADE")
        .execute(&pool)
        .await
        .expect("failed to truncate sessions");
    sqlx::query("DELETE FROM users")
        .execute(&pool)
        .await
        .expect("failed to delete users");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind ephemeral test port");
    let address = listener
        .local_addr()
        .expect("failed to read test listener address");
    let addr_str = format!("http://{address}");

    let app_router = app(pool.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app_router)
            .await
            .expect("test server failed while serving app");
    });

    (addr_str, server, pool)
}

#[tokio::test]
async fn test_full_auth_and_sessions_flow() {
    let (base_url, server_handle, _pool) = setup_test_app().await;
    let client = reqwest::Client::new();

    // 1. Health Status Test
    let health_resp = client
        .get(format!("{}/api/health", base_url))
        .send()
        .await
        .expect("failed to send health request");
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);
    let health_body: HealthStatus = health_resp
        .json()
        .await
        .expect("failed to parse health response");
    assert_eq!(health_body, HealthStatus::ok("backend"));

    // 2. Registration Test (Validation)
    let bad_reg = RegisterRequest {
        username: "".to_string(),
        password: "12".to_string(),
    };
    let bad_reg_resp = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&bad_reg)
        .send()
        .await
        .expect("failed to register invalid user");
    assert_eq!(bad_reg_resp.status(), reqwest::StatusCode::BAD_REQUEST);

    // 3. Registration Test (Success)
    let valid_reg = RegisterRequest {
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };
    let reg_resp = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&valid_reg)
        .send()
        .await
        .expect("failed to register valid user");
    assert_eq!(reg_resp.status(), reqwest::StatusCode::CREATED);
    let reg_body: RegisterResponse = reg_resp
        .json()
        .await
        .expect("failed to parse register response");
    assert!(reg_body.success);

    // 4. Registration Test (Conflict)
    let conflict_reg_resp = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&valid_reg)
        .send()
        .await
        .expect("failed to register conflict user");
    assert_eq!(conflict_reg_resp.status(), reqwest::StatusCode::CONFLICT);

    // 5. Login Test (Invalid Credentials)
    let bad_login = LoginRequest {
        username: "testuser".to_string(),
        password: "wrongpassword".to_string(),
    };
    let bad_login_resp = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&bad_login)
        .send()
        .await
        .expect("failed to login with bad password");
    assert_eq!(bad_login_resp.status(), reqwest::StatusCode::UNAUTHORIZED);

    // 6. Login Test (Success)
    let login_req = LoginRequest {
        username: "testuser".to_string(),
        password: "password123".to_string(),
    };
    let login_resp = client
        .post(format!("{}/api/auth/login", base_url))
        .header("User-Agent", "Test-Browser")
        .json(&login_req)
        .send()
        .await
        .expect("failed to login successfully");
    assert_eq!(login_resp.status(), reqwest::StatusCode::OK);
    let login_body: LoginResponse = login_resp
        .json()
        .await
        .expect("failed to parse login response");
    assert!(login_body.success);
    let token1 = login_body.token.expect("missing token");
    assert_eq!(login_body.username, Some("testuser".to_string()));

    // 7. Login Second Session (Simulate another device)
    let login_resp2 = client
        .post(format!("{}/api/auth/login", base_url))
        .header("User-Agent", "Test-Mobile")
        .json(&login_req)
        .send()
        .await
        .expect("failed second login");
    let login_body2: LoginResponse = login_resp2
        .json()
        .await
        .expect("failed second login response parsing");
    let token2 = login_body2.token.expect("missing token 2");

    // 8. List Sessions (Authenticated with Token 1)
    let sessions_resp = client
        .get(format!("{}/api/sessions", base_url))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .expect("failed to list sessions");
    assert_eq!(sessions_resp.status(), reqwest::StatusCode::OK);
    let sessions: Vec<SessionInfo> = sessions_resp
        .json()
        .await
        .expect("failed to parse sessions list");

    // We should have 2 active sessions
    assert_eq!(sessions.len(), 2);

    // Check that is_current is set correctly
    let current_session = sessions
        .iter()
        .find(|s| s.is_current)
        .expect("should find current session");
    assert_eq!(current_session.token, token1);
    assert_eq!(current_session.user_agent, Some("Test-Browser".to_string()));

    let other_session = sessions
        .iter()
        .find(|s| !s.is_current)
        .expect("should find other session");
    assert_eq!(other_session.token, token2);
    assert_eq!(other_session.user_agent, Some("Test-Mobile".to_string()));

    // 9. Revoke Session (Revoke Token 2 using Token 1 session credentials)
    let revoke_resp = client
        .post(format!("{}/api/sessions/revoke", base_url))
        .header("Authorization", format!("Bearer {}", token1))
        .json(&shared_lib::RevokeRequest {
            token: token2.clone(),
        })
        .send()
        .await
        .expect("failed to send revoke request");
    assert_eq!(revoke_resp.status(), reqwest::StatusCode::OK);
    let revoke_body: RevokeResponse = revoke_resp
        .json()
        .await
        .expect("failed to parse revoke response");
    assert!(revoke_body.success);

    // 10. Check Session 2 is indeed revoked
    let sessions_resp_after = client
        .get(format!("{}/api/sessions", base_url))
        .header("Authorization", format!("Bearer {}", token1))
        .send()
        .await
        .expect("failed to list sessions after revoke");
    let sessions_after: Vec<SessionInfo> = sessions_resp_after
        .json()
        .await
        .expect("failed parsing list sessions after revoke");
    assert_eq!(sessions_after.len(), 1);
    assert_eq!(sessions_after[0].token, token1);

    // 11. Trying to query sessions with the revoked Token 2 should return 401 Unauthorized
    let invalid_auth_resp = client
        .get(format!("{}/api/sessions", base_url))
        .header("Authorization", format!("Bearer {}", token2))
        .send()
        .await
        .expect("failed to send invalid authenticated request");
    assert_eq!(
        invalid_auth_resp.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );

    // Terminate test server
    server_handle.abort();
    let _ = server_handle.await;
}
