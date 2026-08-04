use core::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::config::Config;
use crate::error::DomainError;

/// Migrations, embedded into the binary at compile time.
///
/// Because they are compiled in, the release image needs no migration files on
/// disk and cannot drift from the binary that runs them.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Open the connection pool.
///
/// Connections are established lazily so a database that is still starting up
/// does not prevent the process from booting; readiness is reported separately
/// by [`ping`], which is what the `/health/ready` probe consults.
pub fn connect(config: &Config) -> Result<PgPool, DomainError> {
    PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        // Recycle idle connections so a proxy or database restart does not
        // leave the pool holding handles that fail on first use.
        .idle_timeout(Duration::from_mins(10))
        .max_lifetime(Duration::from_mins(30))
        .connect_lazy(&config.database_url)
        .map_err(DomainError::Database)
}

/// Apply any pending migrations.
///
/// Running this at startup replaces the previous `CREATE TABLE IF NOT EXISTS`
/// calls, which left no record of what had been applied. `sqlx` takes an
/// advisory lock for the duration, so several replicas booting at once is safe.
pub async fn migrate(pool: &PgPool) -> Result<(), DomainError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| DomainError::Database(sqlx::Error::Migrate(Box::new(e))))
}

/// Check that the database answers. Backs the readiness probe.
pub async fn ping(pool: &PgPool) -> Result<(), DomainError> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(DomainError::Database)?;
    Ok(())
}
