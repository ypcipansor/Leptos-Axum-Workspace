use sqlx::{PgPool, Row};

pub(crate) async fn find_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query("SELECT username, password_hash FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = row {
        let u: String = r.get("username");
        let p: String = r.get("password_hash");
        Ok(Some((u, p)))
    } else {
        Ok(None)
    }
}

pub(crate) async fn exists(pool: &PgPool, username: &str) -> Result<bool, sqlx::Error> {
    let row = sqlx::query("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub(crate) async fn create(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2)")
        .bind(username)
        .bind(password_hash)
        .execute(pool)
        .await?;
    Ok(())
}
