use app_core::{ApiError, ValidationError};

/// Failures produced by the domain layer.
///
/// This type is richer than [`ApiError`]: it keeps the underlying `sqlx::Error`
/// and hashing failures so they can be logged with full detail. The conversion
/// into `ApiError` at the transport boundary is where that detail is dropped,
/// which is what stops a SQL error message or connection string from reaching
/// the browser.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error("invalid username or password")]
    InvalidCredentials,

    #[error("username is already taken")]
    UsernameTaken,

    #[error("request is not authenticated")]
    Unauthenticated,

    #[error("session not found")]
    SessionNotFound,

    #[error("too many attempts")]
    RateLimited,

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("password hashing failed: {0}")]
    PasswordHash(String),

    /// A `spawn_blocking` worker panicked or was cancelled.
    #[error("background task failed: {0}")]
    Join(#[from] tokio::task::JoinError),

    /// A row violated an invariant the schema was supposed to guarantee.
    #[error("stored data is inconsistent: {0}")]
    DataIntegrity(String),
}

impl DomainError {
    /// Whether this should be logged at `error` level and investigated.
    ///
    /// Rejected credentials and taken usernames are ordinary outcomes; database
    /// failures are not.
    #[must_use]
    pub const fn is_internal(&self) -> bool {
        matches!(
            self,
            Self::Database(_) | Self::PasswordHash(_) | Self::Join(_) | Self::DataIntegrity(_)
        )
    }
}

impl From<DomainError> for ApiError {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::Validation(e) => Self::Validation(e),
            DomainError::InvalidCredentials => Self::InvalidCredentials,
            DomainError::UsernameTaken => Self::UsernameTaken,
            DomainError::Unauthenticated => Self::Unauthenticated,
            DomainError::SessionNotFound => Self::SessionNotFound,
            DomainError::RateLimited => Self::RateLimited,

            // Everything below is collapsed deliberately. The caller learns
            // that the request failed and nothing about why; the detail is in
            // the server logs, correlated by request id.
            DomainError::Database(_)
            | DomainError::PasswordHash(_)
            | DomainError::Join(_)
            | DomainError::DataIntegrity(_) => Self::Internal,
        }
    }
}

/// Postgres SQLSTATE for `unique_violation`.
const UNIQUE_VIOLATION: &str = "23505";

impl DomainError {
    /// Reinterpret a unique-constraint violation on the username index as
    /// [`DomainError::UsernameTaken`].
    ///
    /// Relying on the constraint rather than a preceding `SELECT` removes both
    /// a round trip and the race window between the check and the insert, in
    /// which two concurrent registrations could both observe "available".
    pub(crate) fn from_insert_user(error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::Database(db) if db.code().as_deref() == Some(UNIQUE_VIOLATION) => {
                Self::UsernameTaken
            }
            _ => Self::Database(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_outcomes_map_to_their_specific_api_error() {
        assert_eq!(
            ApiError::from(DomainError::InvalidCredentials),
            ApiError::InvalidCredentials
        );
        assert_eq!(
            ApiError::from(DomainError::UsernameTaken),
            ApiError::UsernameTaken
        );
        assert_eq!(
            ApiError::from(DomainError::Unauthenticated),
            ApiError::Unauthenticated
        );
        assert_eq!(
            ApiError::from(DomainError::SessionNotFound),
            ApiError::SessionNotFound
        );
    }

    #[test]
    fn internal_failures_collapse_to_internal() {
        assert_eq!(
            ApiError::from(DomainError::Database(sqlx::Error::PoolTimedOut)),
            ApiError::Internal
        );
        assert_eq!(
            ApiError::from(DomainError::PasswordHash("bad salt".to_owned())),
            ApiError::Internal
        );
        assert_eq!(
            ApiError::from(DomainError::DataIntegrity("username".to_owned())),
            ApiError::Internal
        );
    }

    #[test]
    fn internal_error_text_never_reaches_the_client() {
        let error = DomainError::Database(sqlx::Error::Configuration(
            "postgres://user:hunter2@host/db is unreachable".into(),
        ));
        let rendered = ApiError::from(error).to_string();
        assert!(!rendered.contains("hunter2"));
        assert!(!rendered.contains("postgres://"));
    }

    #[test]
    fn classifies_which_failures_deserve_investigation() {
        assert!(DomainError::Database(sqlx::Error::PoolClosed).is_internal());
        assert!(DomainError::PasswordHash(String::new()).is_internal());
        assert!(!DomainError::InvalidCredentials.is_internal());
        assert!(!DomainError::UsernameTaken.is_internal());
    }

    #[test]
    fn validation_failures_keep_their_detail() {
        let error =
            DomainError::Validation(ValidationError::UsernameTooShort { min: 3, actual: 1 });
        assert_eq!(
            ApiError::from(error),
            ApiError::Validation(ValidationError::UsernameTooShort { min: 3, actual: 1 })
        );
    }

    #[test]
    fn non_unique_database_errors_are_not_mistaken_for_a_taken_username() {
        let error = DomainError::from_insert_user(sqlx::Error::PoolTimedOut);
        assert!(matches!(error, DomainError::Database(_)));
    }
}
