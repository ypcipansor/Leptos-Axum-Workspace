use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DomainError;

/// A row of `users`, in database terms.
///
/// Deliberately built from primitives rather than the validated newtypes in
/// `app-core`: the repository's job is to reflect what is stored, and the
/// service layer decides what to make of it. That keeps a single unparseable
/// legacy row from making the whole query fail to decode.
#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Insert a new user.
///
/// Uniqueness is left to the `users_username_lower_key` index rather than
/// checked with a preceding `SELECT`. That removes a round trip and, more
/// importantly, the window in which two concurrent registrations both see the
/// name as available.
pub async fn insert(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
) -> Result<UserRecord, DomainError> {
    sqlx::query_as!(
        UserRecord,
        r#"
        INSERT INTO users (username, password_hash)
        VALUES ($1, $2)
        RETURNING id, username, password_hash, created_at
        "#,
        username,
        password_hash,
    )
    .fetch_one(pool)
    .await
    .map_err(DomainError::from_insert_user)
}

/// Look a user up by name, case-insensitively.
///
/// Matches the functional unique index, so `Alice` finds the account
/// registered as `alice` instead of silently reporting no such user.
pub async fn find_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<UserRecord>, DomainError> {
    sqlx::query_as!(
        UserRecord,
        r#"
        SELECT id, username, password_hash, created_at
        FROM users
        WHERE lower(username) = lower($1)
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .map_err(DomainError::Database)
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserRecord>, DomainError> {
    sqlx::query_as!(
        UserRecord,
        r#"
        SELECT id, username, password_hash, created_at
        FROM users
        WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(pool)
    .await
    .map_err(DomainError::Database)
}
