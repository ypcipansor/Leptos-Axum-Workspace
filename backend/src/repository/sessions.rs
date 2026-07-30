use sqlx::{PgPool, Row};

pub(crate) async fn create(
    pool: &PgPool,
    token: &str,
    session_id: &str,
    username: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sessions (token, session_id, username, user_agent, ip_address) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token)
    .bind(session_id)
    .bind(username)
    .bind(user_agent)
    .bind(ip_address)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn lookup_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query("SELECT username, session_id FROM sessions WHERE token = $1")
        .bind(token)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        let u: String = r.get("username");
        let s: String = r.get("session_id");
        Ok(Some((u, s)))
    } else {
        Ok(None)
    }
}

pub(crate) async fn list_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    sqlx::query(
        "SELECT session_id, username, user_agent, ip_address, created_at FROM sessions WHERE username = $1 ORDER BY created_at DESC",
    )
    .bind(username)
    .fetch_all(pool)
    .await
}

pub(crate) async fn delete(
    pool: &PgPool,
    session_id: &str,
    username: &str,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM sessions WHERE session_id = $1 AND username = $2")
        .bind(session_id)
        .bind(username)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
