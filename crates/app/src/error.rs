use app_core::ApiError;
// Reached through leptos rather than as a direct dependency, so the server_fn
// version can never drift out of step with the leptos version that consumes it.
use leptos::server_fn::error::{FromServerFnError, ServerFnErrorErr};
use serde::{Deserialize, Serialize};

/// The error type every server function in this crate returns.
///
/// It wraps the shared [`ApiError`] and adds one variant for failures that
/// happen below the application: a dropped connection, a serialization
/// mismatch, a middleware rejection. Those are transport problems, not
/// application outcomes, and conflating them would tell a user their password
/// was wrong when their network was simply down.
///
/// `ApiError` itself lives in `app-core` and cannot implement
/// [`FromServerFnError`], because that would drag `server_fn` into a crate that
/// is meant to hold nothing but plain data. Wrapping it here keeps that
/// boundary intact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error("Could not reach the server. Check your connection and try again.")]
    Transport(String),
}

impl FromServerFnError for AppError {
    type Encoder = leptos::server_fn::codec::JsonEncoding;

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        Self::Transport(value.to_string())
    }
}

// Lets a server function write `Username::parse(&raw)?` directly, without
// spelling out the ValidationError -> ApiError -> AppError chain each time.
impl From<app_core::ValidationError> for AppError {
    fn from(error: app_core::ValidationError) -> Self {
        Self::Api(ApiError::Validation(error))
    }
}

impl AppError {
    /// The message to show the user.
    #[must_use]
    pub fn user_message(&self) -> String {
        self.to_string()
    }

    /// Whether this should send the viewer to the sign-in page.
    #[must_use]
    pub const fn is_unauthenticated(&self) -> bool {
        matches!(self, Self::Api(e) if e.is_unauthenticated())
    }

    /// The underlying application error, if this was not a transport failure.
    #[must_use]
    pub const fn as_api(&self) -> Option<&ApiError> {
        match self {
            Self::Api(e) => Some(e),
            Self::Transport(_) => None,
        }
    }
}

/// Convert a domain-layer failure into the wire error, logging anything
/// unexpected on the way through.
///
/// This is the single point where internal detail is dropped. Everything the
/// operator needs stays in the log, correlated by request id; everything that
/// crosses to the browser is safe for an unauthenticated stranger to read.
#[cfg(feature = "ssr")]
#[must_use]
pub fn from_domain(error: app_domain::DomainError) -> AppError {
    if error.is_internal() {
        tracing::error!(error = %error, "request failed");
    } else {
        tracing::debug!(error = %error, "request rejected");
    }

    AppError::Api(ApiError::from(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_core::ValidationError;

    #[test]
    fn wraps_api_errors_without_losing_their_message() {
        let error = AppError::from(ApiError::InvalidCredentials);
        assert_eq!(error.user_message(), "Invalid username or password.");
    }

    #[test]
    fn transport_failures_are_distinguishable_from_application_outcomes() {
        let transport = AppError::Transport("connection reset".to_owned());
        assert!(transport.as_api().is_none());
        assert!(!transport.is_unauthenticated());

        // The raw cause must not be shown to the user as though it were an
        // answer about their credentials.
        assert!(!transport.user_message().contains("connection reset"));
    }

    #[test]
    fn recognises_when_the_viewer_must_sign_in() {
        assert!(AppError::from(ApiError::Unauthenticated).is_unauthenticated());
        assert!(!AppError::from(ApiError::UsernameTaken).is_unauthenticated());
    }

    #[test]
    fn transport_errors_are_constructed_from_server_fn_failures() {
        let error = AppError::from_server_fn_error(ServerFnErrorErr::Serialization("bad".into()));
        assert!(matches!(error, AppError::Transport(_)));
    }

    #[test]
    fn round_trips_through_json() {
        for error in [
            AppError::from(ApiError::InvalidCredentials),
            AppError::from(ApiError::Validation(ValidationError::UsernameInvalidStart)),
            AppError::Transport("timeout".to_owned()),
        ] {
            let json = serde_json::to_string(&error).expect("serialize");
            assert_eq!(
                serde_json::from_str::<AppError>(&json).expect("deserialize"),
                error
            );
        }
    }
}
