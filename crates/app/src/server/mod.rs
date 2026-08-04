//! Server functions: the entire client/server contract.
//!
//! Each `#[server]` function compiles into two things -- an HTTP handler on the
//! server and a typed call on the client -- from one definition. That is what
//! removes the whole hand-written REST layer this template used to carry:
//! no URL literals, no manual `serde_json`, no `Authorization` header
//! assembled in three places, no response envelope to unwrap.
//!
//! Changing an argument or a return type is now a compile error on both sides
//! at once, which is the property a hand-written fetch call can never have.

pub mod auth;

// Both halves of each server function are re-exported: the callable `fn` and
// the struct the `#[server]` macro derives from it. Components need the struct
// to drive a `ServerAction`, which is what lets `<ActionForm>` post to it as a
// plain HTML form when JavaScript is unavailable.
pub use auth::{
    CurrentUser, ListSessions, RevokeSession, SignIn, SignOut, SignUp, current_user, list_sessions,
    revoke_session, sign_in, sign_out, sign_up,
};

/// Helpers available only when compiling for the server.
///
/// Nothing in here is reachable from the browser build, so the database pool,
/// the cookie construction and the session lookup are simply absent from the
/// wasm binary rather than being dead code inside it.
#[cfg(feature = "ssr")]
pub(crate) mod ssr {
    use std::net::SocketAddr;

    use app_core::ApiError;
    use app_domain::AppState;
    use app_domain::auth::{ClientContext, CurrentUser, SessionToken, cookie, resolve_context};
    use axum::extract::ConnectInfo;
    use http::request::Parts;
    use leptos::prelude::*;
    use leptos_axum::ResponseOptions;

    use crate::error::{AppError, from_domain};

    /// The shared application state, provided by the server crate per request.
    pub(crate) fn state() -> Result<AppState, AppError> {
        use_context::<AppState>().ok_or_else(|| {
            // Reaching this means the router was assembled without
            // `provide_context(state)`. It is a wiring bug, not a user error.
            tracing::error!("AppState is missing from the server function context");
            AppError::Api(ApiError::Internal)
        })
    }

    /// The incoming request head. `leptos_axum` provides it automatically.
    pub(crate) fn parts() -> Result<Parts, AppError> {
        use_context::<Parts>().ok_or_else(|| {
            tracing::error!("request Parts are missing from the server function context");
            AppError::Api(ApiError::Internal)
        })
    }

    fn response_options() -> Result<ResponseOptions, AppError> {
        use_context::<ResponseOptions>().ok_or_else(|| {
            tracing::error!("ResponseOptions are missing from the server function context");
            AppError::Api(ApiError::Internal)
        })
    }

    /// Read the session token out of the request's `Cookie` header.
    ///
    /// Returns `None` for an anonymous visitor, which is an ordinary state
    /// rather than an error.
    pub(crate) fn session_token(state: &AppState) -> Option<SessionToken> {
        let parts = parts().ok()?;
        let header = parts.headers.get(http::header::COOKIE)?.to_str().ok()?;
        cookie::extract(header, &state.config)
    }

    /// Resolve the caller, or fail with [`ApiError::Unauthenticated`].
    ///
    /// Every protected server function starts here, so the authorization check
    /// cannot be forgotten in one handler and remembered in another.
    pub(crate) async fn require_user(state: &AppState) -> Result<CurrentUser, AppError> {
        let token = session_token(state).ok_or(AppError::Api(ApiError::Unauthenticated))?;
        state.auth.authenticate(&token).await.map_err(from_domain)
    }

    /// Who is making this request, for the session audit trail.
    pub(crate) fn client_context(state: &AppState) -> ClientContext {
        let Ok(parts) = parts() else {
            return ClientContext::default();
        };

        // Present only when the server is mounted with
        // `into_make_service_with_connect_info::<SocketAddr>()`.
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| *addr);

        resolve_context(&parts.headers, peer, &state.config)
    }

    /// Attach the session cookie to the response.
    pub(crate) fn set_session_cookie(
        state: &AppState,
        token: &SessionToken,
    ) -> Result<(), AppError> {
        let value = cookie::build(token, &state.config);
        append_set_cookie(&value)
    }

    /// Attach a cookie that removes the session cookie.
    pub(crate) fn clear_session_cookie(state: &AppState) -> Result<(), AppError> {
        let value = cookie::clearing(&state.config);
        append_set_cookie(&value)
    }

    fn append_set_cookie(value: &str) -> Result<(), AppError> {
        let header = value.parse().map_err(|e| {
            tracing::error!(error = %e, "constructed an invalid Set-Cookie header");
            AppError::Api(ApiError::Internal)
        })?;

        // `append`, not `insert`: replacing the header would discard any other
        // cookie a future middleware wants to set on the same response.
        response_options()?.append_header(http::header::SET_COOKIE, header);
        Ok(())
    }
}
