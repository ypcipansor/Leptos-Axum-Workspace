use serde::{Deserialize, Serialize};

use crate::validation::ValidationError;

/// The error contract between the server and the browser.
///
/// Server functions return this; the UI matches on it to decide what to render.
/// Because it is an enum rather than a string, adding a case the UI has not
/// handled is a compile error rather than a silently unhelpful message.
///
/// Every variant is safe to display to an unauthenticated stranger. Internal
/// detail -- SQL state, connection strings, backtraces -- is logged server-side
/// and collapsed into [`ApiError::Internal`] before it crosses the wire. The
/// previous implementation returned the same `"Internal server error."` string
/// with a `200`-shaped body, which the UI could not distinguish from success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "code", content = "detail", rename_all = "snake_case")]
pub enum ApiError {
    /// Input failed the shared newtype rules.
    #[error("{0}")]
    Validation(#[from] ValidationError),

    /// Wrong username or wrong password -- deliberately indistinguishable, so
    /// the response cannot be used to enumerate which usernames exist.
    #[error("Invalid username or password.")]
    InvalidCredentials,

    #[error("That username is already taken.")]
    UsernameTaken,

    /// No valid session cookie. The UI redirects to the sign-in page.
    #[error("You need to sign in to continue.")]
    Unauthenticated,

    #[error("That session no longer exists.")]
    SessionNotFound,

    #[error("Too many attempts. Please wait a moment and try again.")]
    RateLimited,

    #[error("Something went wrong on our end. Please try again.")]
    Internal,
}

impl ApiError {
    /// Whether the UI should send the viewer to the sign-in page.
    #[must_use]
    pub const fn is_unauthenticated(&self) -> bool {
        matches!(self, Self::Unauthenticated)
    }

    /// Whether retrying the same request unchanged could plausibly succeed.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Internal | Self::RateLimited)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        for error in [
            ApiError::InvalidCredentials,
            ApiError::UsernameTaken,
            ApiError::Unauthenticated,
            ApiError::SessionNotFound,
            ApiError::RateLimited,
            ApiError::Internal,
            ApiError::Validation(ValidationError::UsernameTooShort { min: 3, actual: 1 }),
        ] {
            let json = serde_json::to_string(&error).unwrap();
            assert_eq!(serde_json::from_str::<ApiError>(&json).unwrap(), error);
        }
    }

    #[test]
    fn credential_failure_message_does_not_reveal_which_half_was_wrong() {
        let message = ApiError::InvalidCredentials.to_string();
        assert!(!message.to_lowercase().contains("not found"));
        assert!(!message.to_lowercase().contains("no such user"));
        assert_eq!(message, "Invalid username or password.");
    }

    #[test]
    fn validation_errors_keep_their_specific_message() {
        let error = ApiError::from(ValidationError::PasswordTooShort { min: 12, actual: 4 });
        assert_eq!(
            error.to_string(),
            "Password must be at least 12 characters (got 4)."
        );
    }

    #[test]
    fn classifies_unauthenticated_and_retryable_cases() {
        assert!(ApiError::Unauthenticated.is_unauthenticated());
        assert!(!ApiError::Internal.is_unauthenticated());

        assert!(ApiError::Internal.is_retryable());
        assert!(ApiError::RateLimited.is_retryable());
        assert!(!ApiError::InvalidCredentials.is_retryable());
        assert!(!ApiError::UsernameTaken.is_retryable());
    }
}
