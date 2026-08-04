//! End-to-end coverage of the authentication slice, exercised over real HTTP
//! against the same router the binary serves.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use core::time::Duration;

use app_core::SessionSummary;
use common::TestApp;
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, LOCATION, SET_COOKIE};

const PASSWORD: &str = "correct horse battery staple";

/// Server functions serialize `AppError`, so what crosses the wire is the
/// variant tag from `#[serde(tag = "code")]` -- `unauthenticated`,
/// `rate_limited` -- not the human-readable Display text. Asserting on the tag
/// keeps these tests pinned to the actual contract rather than to wording that
/// may be reworded for the UI.
fn assert_error_code(body: &str, code: &str) {
    assert!(
        body.contains(code),
        "expected error code `{code}` in response body: {body}"
    );
}

// ---------------------------------------------------------------------------
// Server-side rendering
//
// The single clearest difference from the previous implementation: these pages
// arrive as complete HTML. Before, every route returned an empty <body> and the
// content only existed after a wasm bundle downloaded and executed.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sign_in_page_is_fully_rendered_by_the_server() {
    let app = TestApp::spawn().await;

    let body = app
        .client()
        .get(app.url("/signin"))
        .header(ACCEPT, "text/html")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    // Content a crawler or a reader without JavaScript can actually see.
    assert!(
        body.contains("Sign in"),
        "page content missing from: {body}"
    );
    assert!(body.contains("<form"), "form missing from: {body}");
    assert!(body.contains("name=\"username\""), "username field missing");
    assert!(body.contains("name=\"password\""), "password field missing");
}

#[tokio::test]
async fn pages_carry_a_localised_title_and_language() {
    let app = TestApp::spawn().await;

    let body = app
        .client()
        .get(app.url("/signup"))
        .header(ACCEPT, "text/html")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(body.contains("<html lang=\"en\""), "lang attribute missing");
    assert!(
        body.contains("Create an account"),
        "title missing from: {body}"
    );
}

#[tokio::test]
async fn an_anonymous_visitor_is_redirected_away_from_the_dashboard() {
    let app = TestApp::spawn().await;

    let response = app
        .client()
        .get(app.url("/"))
        .header(ACCEPT, "text/html")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers().get(LOCATION).unwrap(), "/signin");
}

#[tokio::test]
async fn an_unknown_route_reports_not_found() {
    let app = TestApp::spawn().await;

    let response = app
        .client()
        .get(app.url("/no-such-page"))
        .header(ACCEPT, "text/html")
        .send()
        .await
        .unwrap();

    // A 200 with apologetic prose, which is what the previous router produced
    // for any unrecognised path, is indistinguishable from success to a crawler.
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Health probes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn liveness_answers_without_touching_the_database() {
    let app = TestApp::spawn().await;

    let response = app
        .client()
        .get(app.url("/health/live"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn readiness_reports_the_database_state() {
    let app = TestApp::spawn().await;

    let response = app
        .client()
        .get(app.url("/health/ready"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["database"], "up");
    assert!(body["version"].is_string());
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn signing_up_creates_an_account_and_starts_a_session() {
    let app = TestApp::spawn().await;
    let client = app.client();

    let response = app.sign_up(&client, "alice", PASSWORD).await;
    assert!(response.status().is_success(), "got {}", response.status());
    assert_eq!(response.headers().get(LOCATION).unwrap(), "/");
    assert_eq!(app.session_count("alice").await, 1);
}

#[tokio::test]
async fn the_session_cookie_carries_every_hardening_attribute() {
    let app = TestApp::spawn().await;
    let client = app.client();

    let response = app.sign_up(&client, "alice", PASSWORD).await;
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .expect("no session cookie was set")
        .to_str()
        .unwrap();

    // HttpOnly is the attribute that makes the token unreadable from
    // JavaScript. The previous implementation returned the token in a JSON
    // body and the browser stored it in localStorage, where any injected
    // script could read it.
    assert!(cookie.contains("HttpOnly"), "got {cookie}");
    assert!(cookie.contains("SameSite=Lax"), "got {cookie}");
    assert!(cookie.contains("Path=/"), "got {cookie}");
    assert!(cookie.contains("Max-Age="), "got {cookie}");
}

#[tokio::test]
async fn the_session_token_is_never_stored_in_a_usable_form() {
    let app = TestApp::spawn().await;
    let client = app.client();

    let response = app.sign_up(&client, "alice", PASSWORD).await;
    let set_cookie = response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let token = set_cookie
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_owned();
    assert!(!token.is_empty());

    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM sessions LIMIT 1")
        .fetch_one(&app.pool)
        .await
        .unwrap();

    // A dump of this table must not yield anything presentable as a credential.
    assert_eq!(stored.len(), 32, "expected a SHA-256 digest");
    assert_ne!(
        stored,
        token.as_bytes(),
        "the raw token was stored verbatim"
    );
    assert!(
        !String::from_utf8_lossy(&stored).contains(&token),
        "the stored value contains the token"
    );
}

#[tokio::test]
async fn the_password_is_stored_as_an_argon2id_hash() {
    let app = TestApp::spawn().await;
    let client = app.client();
    app.sign_up(&client, "alice", PASSWORD).await;

    let hash: String =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE username = 'alice'")
            .fetch_one(&app.pool)
            .await
            .unwrap();

    assert!(hash.starts_with("$argon2id$"), "got {hash}");
    assert!(!hash.contains(PASSWORD));
}

#[tokio::test]
async fn a_taken_username_is_rejected() {
    let app = TestApp::spawn().await;

    app.sign_up(&app.client(), "alice", PASSWORD).await;
    let response = app.sign_up(&app.client(), "alice", PASSWORD).await;

    assert!(!response.status().is_success(), "duplicate was accepted");
    assert_eq!(app.session_count("alice").await, 1);
}

#[tokio::test]
async fn usernames_are_taken_case_insensitively() {
    let app = TestApp::spawn().await;

    app.sign_up(&app.client(), "alice", PASSWORD).await;
    let response = app.sign_up(&app.client(), "ALICE", PASSWORD).await;

    // Without the lower(username) unique index these would be two accounts a
    // user could not tell apart.
    assert!(!response.status().is_success(), "case variant was accepted");
}

#[tokio::test]
async fn registration_enforces_the_shared_validation_rules() {
    let app = TestApp::spawn().await;

    for (username, password, case) in [
        ("ab", PASSWORD, "username too short"),
        ("has space", PASSWORD, "username has a space"),
        (".leading", PASSWORD, "username starts with punctuation"),
        ("alice", "short", "password too short"),
    ] {
        let response = app.sign_up(&app.client(), username, password).await;
        assert!(
            !response.status().is_success(),
            "{case} was accepted with status {}",
            response.status()
        );
    }
}

// ---------------------------------------------------------------------------
// Sign-in
// ---------------------------------------------------------------------------

#[tokio::test]
async fn signing_in_with_valid_credentials_starts_a_second_session() {
    let app = TestApp::spawn().await;
    app.sign_up(&app.client(), "alice", PASSWORD).await;

    let response = app.sign_in(&app.client(), "alice", PASSWORD).await;

    assert!(response.status().is_success(), "got {}", response.status());
    assert!(response.headers().contains_key(SET_COOKIE));
    assert_eq!(app.session_count("alice").await, 2);
}

#[tokio::test]
async fn signing_in_works_regardless_of_username_case() {
    let app = TestApp::spawn().await;
    app.sign_up(&app.client(), "alice", PASSWORD).await;

    let response = app.sign_in(&app.client(), "ALICE", PASSWORD).await;
    assert!(response.status().is_success(), "got {}", response.status());
}

#[tokio::test]
async fn a_wrong_password_is_rejected_without_starting_a_session() {
    let app = TestApp::spawn().await;
    app.sign_up(&app.client(), "alice", PASSWORD).await;

    let response = app
        .sign_in(&app.client(), "alice", "wrong password here")
        .await;

    assert!(!response.status().is_success());
    assert!(!response.headers().contains_key(SET_COOKIE));
    assert_eq!(app.session_count("alice").await, 1);
}

#[tokio::test]
async fn an_unknown_user_and_a_wrong_password_are_indistinguishable() {
    let app = TestApp::spawn().await;
    app.sign_up(&app.client(), "alice", PASSWORD).await;

    let wrong_password = app
        .sign_in(&app.client(), "alice", "wrong password here")
        .await;
    let unknown_user = app.sign_in(&app.client(), "nobody", PASSWORD).await;

    assert_eq!(wrong_password.status(), unknown_user.status());
    assert_eq!(
        wrong_password.text().await.unwrap(),
        unknown_user.text().await.unwrap(),
        "responses differ, which lets an attacker enumerate usernames"
    );
}

#[tokio::test]
async fn repeated_failures_are_rate_limited() {
    let app = TestApp::spawn().await;
    app.sign_up(&app.client(), "alice", PASSWORD).await;
    let client = app.client();

    let mut throttled = false;
    for _ in 0..15 {
        let response = app.sign_in(&client, "alice", "wrong password here").await;
        if response.text().await.unwrap().contains("rate_limited") {
            throttled = true;
            break;
        }
    }

    assert!(throttled, "unlimited guesses were accepted");
}

#[tokio::test]
async fn a_successful_sign_in_is_not_blocked_by_earlier_failures() {
    let app = TestApp::spawn().await;
    app.sign_up(&app.client(), "alice", PASSWORD).await;
    let client = app.client();

    for _ in 0..3 {
        app.sign_in(&client, "alice", "wrong password here").await;
    }

    let response = app.sign_in(&client, "alice", PASSWORD).await;
    assert!(response.status().is_success(), "got {}", response.status());
}

// ---------------------------------------------------------------------------
// Session management
// ---------------------------------------------------------------------------

#[tokio::test]
async fn listing_sessions_requires_authentication() {
    let app = TestApp::spawn().await;

    let response = app.post(&app.client(), "/api/list_sessions").await;
    let body = response.text().await.unwrap();
    assert_error_code(&body, "unauthenticated");
}

#[tokio::test]
async fn the_session_list_marks_the_current_device() {
    let app = TestApp::spawn().await;
    let client = app.signed_in_client("alice", PASSWORD).await;

    // A second sign-in from a different client, so there are two rows and only
    // one of them can be the current one.
    app.sign_in(&app.client(), "alice", PASSWORD).await;

    let body = app
        .post(&client, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    let sessions: Vec<SessionSummary> = serde_json::from_str(&body).unwrap();

    assert_eq!(sessions.len(), 2);
    assert_eq!(
        sessions.iter().filter(|s| s.is_current).count(),
        1,
        "exactly one session should be the current one"
    );
}

#[tokio::test]
async fn revoking_a_session_ends_it_immediately() {
    let app = TestApp::spawn().await;
    let owner = app.signed_in_client("alice", PASSWORD).await;

    let other = app.client();
    app.sign_in(&other, "alice", PASSWORD).await;
    assert_eq!(app.session_count("alice").await, 2);

    let body = app
        .post(&owner, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    let sessions: Vec<SessionSummary> = serde_json::from_str(&body).unwrap();
    let target = sessions.iter().find(|s| !s.is_current).unwrap();

    let response = owner
        .post(app.url("/api/revoke_session"))
        .form(&[("session_id", target.id.to_string())])
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success(), "got {}", response.status());

    assert_eq!(app.session_count("alice").await, 1);

    // The revoked client's cookie must stop working on its very next request,
    // with no window in which an already-issued credential stays valid.
    let after = app
        .post(&other, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    assert_error_code(&after, "unauthenticated");
}

#[tokio::test]
async fn a_session_belonging_to_someone_else_cannot_be_revoked() {
    let app = TestApp::spawn().await;

    let alice = app.signed_in_client("alice", PASSWORD).await;
    let mallory = app.signed_in_client("mallory", PASSWORD).await;

    let body = app
        .post(&alice, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    let alice_sessions: Vec<SessionSummary> = serde_json::from_str(&body).unwrap();
    let victim = alice_sessions.first().unwrap();

    let response = mallory
        .post(app.url("/api/revoke_session"))
        .form(&[("session_id", victim.id.to_string())])
        .send()
        .await
        .unwrap();

    // Reported as "not found" rather than "forbidden", so the response cannot
    // be used to confirm that a session id exists.
    assert_error_code(&response.text().await.unwrap(), "session_not_found");
    assert_eq!(app.session_count("alice").await, 1);
}

#[tokio::test]
async fn signing_out_ends_the_session_and_clears_the_cookie() {
    let app = TestApp::spawn().await;
    let client = app.signed_in_client("alice", PASSWORD).await;

    let response = app.post(&client, "/api/sign_out").await;
    assert!(response.status().is_success(), "got {}", response.status());

    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cookie.contains("Max-Age=0"), "cookie not cleared: {cookie}");

    assert_eq!(app.session_count("alice").await, 0);
}

#[tokio::test]
async fn signing_out_twice_is_not_an_error() {
    let app = TestApp::spawn().await;
    let client = app.signed_in_client("alice", PASSWORD).await;

    app.post(&client, "/api/sign_out").await;
    let second = app.post(&client, "/api/sign_out").await;

    assert!(second.status().is_success(), "got {}", second.status());
}

#[tokio::test]
async fn an_expired_session_no_longer_authenticates() {
    let app = TestApp::spawn().await;
    let client = app.signed_in_client("alice", PASSWORD).await;

    // The row is still present -- the sweep has not run -- so this proves the
    // expiry predicate on the lookup, not the cleanup task.
    app.expire_sessions("alice").await;
    assert_eq!(app.session_count("alice").await, 1);

    let body = app
        .post(&client, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    assert_error_code(&body, "unauthenticated");
}

#[tokio::test]
async fn an_expired_session_is_hidden_from_the_session_list() {
    let app = TestApp::spawn().await;
    // The first session is created here and then expired below; only the
    // second client's session should survive to be listed.
    let _first = app.signed_in_client("alice", PASSWORD).await;

    let other = app.client();
    app.sign_in(&other, "alice", PASSWORD).await;

    app.expire_oldest_session().await;

    let body = app
        .post(&other, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    let sessions: Vec<SessionSummary> = serde_json::from_str(&body).unwrap();
    assert_eq!(sessions.len(), 1, "an expired session was listed");
}

#[tokio::test]
async fn a_session_captures_the_client_that_created_it() {
    let app = TestApp::spawn().await;
    let client = app.client();

    client
        .post(app.url("/api/sign_up"))
        .header(reqwest::header::USER_AGENT, "IntegrationTest/1.0")
        .form(&[("username", "alice"), ("password", PASSWORD)])
        .send()
        .await
        .unwrap();

    let body = app
        .post(&client, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    let sessions: Vec<SessionSummary> = serde_json::from_str(&body).unwrap();

    assert_eq!(
        sessions[0].user_agent.as_deref(),
        Some("IntegrationTest/1.0")
    );
    // Directly connected, no declared proxies: the loopback peer address.
    assert_eq!(sessions[0].ip_address.as_deref(), Some("127.0.0.1"));
}

#[tokio::test]
async fn a_forwarded_header_is_ignored_when_no_proxy_is_declared() {
    let app = TestApp::spawn().await;
    let client = app.client();

    client
        .post(app.url("/api/sign_up"))
        .header("x-forwarded-for", "203.0.113.99")
        .form(&[("username", "alice"), ("password", PASSWORD)])
        .send()
        .await
        .unwrap();

    let body = app
        .post(&client, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    let sessions: Vec<SessionSummary> = serde_json::from_str(&body).unwrap();

    // The caller does not get to choose what is recorded about them.
    assert_eq!(sessions[0].ip_address.as_deref(), Some("127.0.0.1"));
}

#[tokio::test]
async fn a_forwarded_header_is_honoured_when_a_proxy_is_declared() {
    let app = TestApp::spawn_with(|config| config.trusted_proxy_hops = 1).await;
    let client = app.client();

    client
        .post(app.url("/api/sign_up"))
        .header("x-forwarded-for", "203.0.113.99")
        .form(&[("username", "alice"), ("password", PASSWORD)])
        .send()
        .await
        .unwrap();

    let body = app
        .post(&client, "/api/list_sessions")
        .await
        .text()
        .await
        .unwrap();
    let sessions: Vec<SessionSummary> = serde_json::from_str(&body).unwrap();

    assert_eq!(sessions[0].ip_address.as_deref(), Some("203.0.113.99"));
}

// ---------------------------------------------------------------------------
// Transport hardening
// ---------------------------------------------------------------------------

#[tokio::test]
async fn responses_carry_security_headers() {
    let app = TestApp::spawn().await;

    let response = app
        .client()
        .get(app.url("/signin"))
        .header(ACCEPT, "text/html")
        .send()
        .await
        .unwrap();

    let headers = response.headers();
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert!(headers.contains_key("content-security-policy"));
    assert!(headers.contains_key("referrer-policy"));
    // Development: HSTS must not be sent over plain HTTP, or it would pin the
    // developer's browser to HTTPS for localhost across every project.
    assert!(!headers.contains_key("strict-transport-security"));
}

#[tokio::test]
async fn every_response_carries_a_request_id() {
    let app = TestApp::spawn().await;

    let response = app
        .client()
        .get(app.url("/health/live"))
        .send()
        .await
        .unwrap();
    assert!(
        response.headers().contains_key("x-request-id"),
        "no request id to correlate logs with"
    );
}

#[tokio::test]
async fn an_oversized_body_is_refused() {
    let app = TestApp::spawn().await;

    let response = app
        .client()
        .post(app.url("/api/sign_in"))
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body("username=alice&password=".to_owned() + &"a".repeat(512 * 1024))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .unwrap();

    assert!(
        !response.status().is_success(),
        "an unbounded body was accepted"
    );
}
