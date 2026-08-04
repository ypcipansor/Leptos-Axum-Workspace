//! Integration test harness.
//!
//! Each test gets its own freshly migrated database and its own server bound to
//! an ephemeral port, so tests are order-independent and can run in parallel.
//!
//! The previous suite ran every test against one shared database, truncating
//! tables between cases, and started the server with `cargo run -p backend &`
//! followed by a fixed sleep. That made failures depend on timing and on which
//! test ran first.

// `unused_self` fires on `client()`, which is a method for call-site symmetry
// with the rest of the harness rather than because it needs `self`.
#![allow(
    clippy::expect_used,
    clippy::unused_self,
    clippy::unwrap_used,
    dead_code,
    unreachable_pub
)]

use core::time::Duration;
use std::net::{Ipv4Addr, SocketAddr};

use app_domain::{AppState, Config, Environment, db};
use leptos::config::get_configuration;
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use uuid::Uuid;

/// A running server plus the database it owns.
pub struct TestApp {
    pub address: String,
    pub pool: PgPool,
    database_name: String,
    admin_url: String,
}

impl TestApp {
    /// Boot a server with the default configuration.
    pub async fn spawn() -> Self {
        Self::spawn_with(|_| {}).await
    }

    /// Boot a server, adjusting the configuration first.
    ///
    /// Used by tests that need a short session lifetime or a declared proxy
    /// hop count without waiting on real time or standing up a proxy.
    pub async fn spawn_with(customise: impl FnOnce(&mut Config)) -> Self {
        let admin_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/postgres".to_owned());

        let database_name = format!("app_test_{}", Uuid::new_v4().simple());
        create_database(&admin_url, &database_name).await;

        let database_url = replace_database(&admin_url, &database_name);

        let mut config = Config {
            database_url: database_url.clone(),
            database_max_connections: 5,
            bind_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            // Development, so the session cookie is not `Secure` and therefore
            // survives the plain-HTTP request the test client makes.
            environment: Environment::Development,
            session_ttl: Duration::from_hours(1),
            session_cleanup_interval: Duration::from_hours(1),
            trusted_proxy_hops: 0,
        };
        customise(&mut config);

        let pool = db::connect(&config).expect("failed to build the pool");
        db::migrate(&pool).await.expect("migrations failed");

        // Binding port 0 lets the OS pick a free port, so parallel tests never
        // collide and no test has to guess whether a port is in use.
        let listener = tokio::net::TcpListener::bind(config.bind_addr)
            .await
            .expect("failed to bind an ephemeral port");
        let address = format!("http://{}", listener.local_addr().unwrap());

        // Points at the workspace manifest so the [[workspace.metadata.leptos]]
        // block is found regardless of the working directory.
        let leptos_options = get_configuration(Some("../../Cargo.toml"))
            .expect("failed to read the leptos configuration")
            .leptos_options;

        let state = AppState::new(pool.clone(), config);
        let router = server::router::build(state, leptos_options);

        tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("server error");
        });

        Self {
            address,
            pool,
            database_name,
            admin_url,
        }
    }

    /// An HTTP client that keeps cookies, like a browser.
    ///
    /// `redirect(none)` because the assertions care about the redirect itself
    /// -- its status and its `Set-Cookie` -- not about what it points at.
    pub fn client(&self) -> reqwest::Client {
        reqwest::Client::builder()
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("failed to build the HTTP client")
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.address)
    }

    /// Register an account and return a client already holding its session.
    pub async fn signed_in_client(&self, username: &str, password: &str) -> reqwest::Client {
        let client = self.client();
        let response = self.sign_up(&client, username, password).await;
        assert!(
            response.status().is_success() || response.status().is_redirection(),
            "sign-up failed with {}",
            response.status()
        );
        client
    }

    pub async fn sign_up(
        &self,
        client: &reqwest::Client,
        username: &str,
        password: &str,
    ) -> reqwest::Response {
        client
            .post(self.url("/api/sign_up"))
            .form(&[("username", username), ("password", password)])
            .send()
            .await
            .expect("request failed")
    }

    pub async fn sign_in(
        &self,
        client: &reqwest::Client,
        username: &str,
        password: &str,
    ) -> reqwest::Response {
        client
            .post(self.url("/api/sign_in"))
            .form(&[("username", username), ("password", password)])
            .send()
            .await
            .expect("request failed")
    }

    pub async fn post(&self, client: &reqwest::Client, path: &str) -> reqwest::Response {
        client
            .post(self.url(path))
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(String::new())
            .send()
            .await
            .expect("request failed")
    }

    /// Count session rows for a username, whatever their state.
    pub async fn session_count(&self, username: &str) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sessions s
             JOIN users u ON u.id = s.user_id
             WHERE lower(u.username) = lower($1)",
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .expect("query failed")
    }

    /// Age every session belonging to a user until it has expired.
    ///
    /// `created_at` moves back with `expires_at` rather than only the latter.
    /// The schema enforces `expires_at > created_at`, so a session cannot be
    /// made to look as though it expired before it began -- and a genuinely
    /// expired session is an old one, which is what this reproduces. Beats
    /// sleeping through a real timeout, and exercises exactly the condition the
    /// `expires_at > now()` predicates exist to catch.
    pub async fn expire_sessions(&self, username: &str) {
        sqlx::query(
            "UPDATE sessions
             SET created_at   = now() - interval '2 hours',
                 last_seen_at = now() - interval '2 hours',
                 expires_at   = now() - interval '1 hour'
             WHERE user_id IN (SELECT id FROM users WHERE lower(username) = lower($1))",
        )
        .bind(username)
        .execute(&self.pool)
        .await
        .expect("query failed");
    }

    /// Age only the oldest session of a user, leaving newer ones valid.
    pub async fn expire_oldest_session(&self) {
        sqlx::query(
            "UPDATE sessions
             SET created_at   = now() - interval '2 hours',
                 last_seen_at = now() - interval '2 hours',
                 expires_at   = now() - interval '1 hour'
             WHERE id = (SELECT id FROM sessions ORDER BY created_at LIMIT 1)",
        )
        .execute(&self.pool)
        .await
        .expect("query failed");
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Best effort. A leaked database from a hard abort is harmless in a
        // throwaway CI container, and failing here would mask the real failure.
        let admin_url = self.admin_url.clone();
        let name = self.database_name.clone();

        std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            runtime.block_on(async move {
                if let Ok(mut conn) = PgConnection::connect(&admin_url).await {
                    // Same audit as in `create_database`: `name` is the
                    // UUID-derived value this struct was constructed with.
                    let statement = format!(r#"DROP DATABASE IF EXISTS "{name}" WITH (FORCE)"#);
                    let _ = sqlx::raw_sql(AssertSqlSafe(statement))
                        .execute(&mut conn)
                        .await;
                }
            });
        })
        .join()
        .ok();
    }
}

async fn create_database(admin_url: &str, name: &str) {
    let mut conn = PgConnection::connect(admin_url)
        .await
        .expect("could not reach the test database server; is TEST_DATABASE_URL correct?");

    // `raw_sql` is the documented path for DDL: CREATE DATABASE cannot run as a
    // prepared statement, and a database name cannot be a bind parameter.
    //
    // sqlx 0.9 requires dynamic SQL to be wrapped in `AssertSqlSafe`, which is
    // the audit this comment records: `name` is `format!("app_test_{}", uuid)`
    // built a few lines above and never derived from test input or anything
    // else outside this file, so there is no path by which it could carry
    // injected SQL.
    let statement = format!(r#"CREATE DATABASE "{name}""#);
    sqlx::raw_sql(AssertSqlSafe(statement))
        .execute(&mut conn)
        .await
        .expect("failed to create the test database");
}

fn replace_database(url: &str, name: &str) -> String {
    let (base, _) = url.rsplit_once('/').expect("database URL has no path");
    format!("{base}/{name}")
}
