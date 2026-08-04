use std::net::IpAddr;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::token::TokenHash;
use crate::error::DomainError;

/// A row of `sessions`.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_agent: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// A session joined with the user it belongs to.
///
/// Authenticating a request needs both, and fetching them together keeps the
/// hot path to a single round trip.
#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub user_created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Client details captured when a session is created, for the session list.
#[derive(Debug, Clone, Default)]
pub struct ClientContext {
    pub user_agent: Option<String>,
    /// `None` when the address is unknown or arrived over an untrusted hop.
    /// Recorded honestly rather than back-filled with a placeholder, which is
    /// what the previous implementation did with `127.0.0.1`.
    pub ip_address: Option<IpAddr>,
}

/// Create a session valid for `ttl_seconds` from now.
///
/// `expires_at` is computed by Postgres rather than in Rust so that it shares a
/// clock with `created_at`. Deriving it application-side risks tripping the
/// `expires_at > created_at` constraint whenever the app and database clocks
/// drift apart.
pub async fn insert(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &TokenHash,
    client: &ClientContext,
    ttl_seconds: f64,
) -> Result<SessionRecord, DomainError> {
    sqlx::query_as!(
        SessionRecord,
        r#"
        INSERT INTO sessions (user_id, token_hash, user_agent, ip_address, expires_at)
        VALUES ($1, $2, $3, $4, now() + make_interval(secs => $5))
        RETURNING
            id,
            user_id,
            user_agent,
            ip_address AS "ip_address: IpAddr",
            created_at,
            last_seen_at,
            expires_at
        "#,
        user_id,
        token_hash.as_bytes(),
        client.user_agent.as_deref(),
        client.ip_address as Option<IpAddr>,
        ttl_seconds,
    )
    .fetch_one(pool)
    .await
    .map_err(DomainError::Database)
}

/// Resolve a token hash to its session and user, if the session is still valid.
///
/// The expiry check lives in the `WHERE` clause: an expired session is
/// indistinguishable from a missing one, so a stale cookie cannot authenticate
/// a request even if the cleanup sweep has not run yet.
pub async fn find_active_by_token_hash(
    pool: &PgPool,
    token_hash: &TokenHash,
) -> Result<Option<AuthenticatedSession>, DomainError> {
    sqlx::query_as!(
        AuthenticatedSession,
        r#"
        SELECT
            s.id        AS session_id,
            u.id        AS user_id,
            u.username  AS username,
            u.created_at AS user_created_at,
            s.expires_at AS expires_at
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1
          AND s.expires_at > now()
        "#,
        token_hash.as_bytes(),
    )
    .fetch_optional(pool)
    .await
    .map_err(DomainError::Database)
}

/// Record activity and slide the expiry window forward.
///
/// Called after a request authenticates, so an actively used session does not
/// expire mid-use while an abandoned one still ages out.
///
/// The `last_seen_at` predicate throttles the write: without it every
/// authenticated request would issue an `UPDATE`, turning a read-only page view
/// into a write on the hottest table in the schema. Sliding at most once per
/// `min_idle_seconds` keeps the same behaviour at a fraction of the cost.
pub async fn touch(
    pool: &PgPool,
    session_id: Uuid,
    ttl_seconds: f64,
    min_idle_seconds: f64,
) -> Result<(), DomainError> {
    sqlx::query!(
        r#"
        UPDATE sessions
        SET last_seen_at = now(),
            expires_at   = now() + make_interval(secs => $2)
        WHERE id = $1
          AND last_seen_at <= now() - make_interval(secs => $3)
        "#,
        session_id,
        ttl_seconds,
        min_idle_seconds,
    )
    .execute(pool)
    .await
    .map_err(DomainError::Database)?;

    Ok(())
}

/// All non-expired sessions for a user, newest first.
pub async fn list_active_for_user(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<SessionRecord>, DomainError> {
    sqlx::query_as!(
        SessionRecord,
        r#"
        SELECT
            id,
            user_id,
            user_agent,
            ip_address AS "ip_address: IpAddr",
            created_at,
            last_seen_at,
            expires_at
        FROM sessions
        WHERE user_id = $1
          AND expires_at > now()
        ORDER BY created_at DESC
        "#,
        user_id,
    )
    .fetch_all(pool)
    .await
    .map_err(DomainError::Database)
}

/// Delete one session, but only if it belongs to `user_id`.
///
/// The ownership predicate is part of the statement rather than a preceding
/// check, so there is no window in which it could be bypassed. Returns the
/// number of rows removed: zero means "not yours or not there", and the caller
/// cannot tell those apart -- which is what stops this from being used to probe
/// for other users' session ids.
pub async fn delete_for_user(
    pool: &PgPool,
    session_id: Uuid,
    user_id: Uuid,
) -> Result<u64, DomainError> {
    let result = sqlx::query!(
        "DELETE FROM sessions WHERE id = $1 AND user_id = $2",
        session_id,
        user_id,
    )
    .execute(pool)
    .await
    .map_err(DomainError::Database)?;

    Ok(result.rows_affected())
}

/// Delete the session identified by a token hash. Used on sign-out.
pub async fn delete_by_token_hash(
    pool: &PgPool,
    token_hash: &TokenHash,
) -> Result<u64, DomainError> {
    let result = sqlx::query!(
        "DELETE FROM sessions WHERE token_hash = $1",
        token_hash.as_bytes(),
    )
    .execute(pool)
    .await
    .map_err(DomainError::Database)?;

    Ok(result.rows_affected())
}

/// Delete every session for a user. Used by "sign out everywhere".
pub async fn delete_all_for_user(pool: &PgPool, user_id: Uuid) -> Result<u64, DomainError> {
    let result = sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .map_err(DomainError::Database)?;

    Ok(result.rows_affected())
}

/// Remove expired rows. Run periodically by the background sweep.
///
/// Expired sessions are already unusable thanks to the `WHERE` clauses above;
/// this only stops the table from growing without bound.
pub async fn delete_expired(pool: &PgPool) -> Result<u64, DomainError> {
    let result = sqlx::query!("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(pool)
        .await
        .map_err(DomainError::Database)?;

    Ok(result.rows_affected())
}
