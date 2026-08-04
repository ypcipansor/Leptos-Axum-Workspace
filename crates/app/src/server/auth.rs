use app_core::{SessionSummary, UserProfile};
use leptos::prelude::*;
use uuid::Uuid;

use crate::error::AppError;

/// Create an account and sign the new user in.
///
/// Registering and then asking the user to sign in again would be a pointless
/// extra step, so this issues a session immediately, exactly as a successful
/// [`sign_in`] does.
///
/// Arguments are plain `String`s rather than the validated newtypes so that
/// `<ActionForm>` can post this as an ordinary HTML form -- the browser sends
/// strings, and a nested struct would force an awkward `credentials[username]`
/// field naming. Validation still happens in exactly one place: the shared
/// parsers in `app-core`, invoked below.
#[server(endpoint = "sign_up")]
pub async fn sign_up(username: String, password: String) -> Result<(), AppError> {
    use app_core::{Credentials, Password, Username};

    use crate::error::from_domain;
    use crate::server::ssr;

    let credentials = Credentials {
        username: Username::parse(&username)?,
        password: Password::parse(&password)?,
    };

    let state = ssr::state()?;
    state
        .auth
        .register(credentials.clone())
        .await
        .map_err(from_domain)?;

    let client = ssr::client_context(&state);
    let (token, profile) = state
        .auth
        .sign_in(credentials, client)
        .await
        .map_err(from_domain)?;

    ssr::set_session_cookie(&state, &token)?;
    tracing::info!(user_id = %profile.id, "account created");

    // A server-side redirect rather than a client-side navigation, so the flow
    // also works with JavaScript disabled.
    leptos_axum::redirect("/");
    Ok(())
}

/// Verify credentials and start a session.
#[server(endpoint = "sign_in")]
pub async fn sign_in(username: String, password: String) -> Result<(), AppError> {
    use app_core::{Credentials, Password, Username};

    use crate::error::from_domain;
    use crate::server::ssr;

    // Note the deliberate asymmetry with `sign_up`: a malformed username here
    // yields the same generic rejection as a wrong password, because reporting
    // "that username is too short to exist" would confirm which names are
    // absent from the database.
    let credentials = Credentials {
        username: Username::parse(&username).map_err(|_| app_core::ApiError::InvalidCredentials)?,
        password: Password::parse(&password).map_err(|_| app_core::ApiError::InvalidCredentials)?,
    };

    let state = ssr::state()?;
    let client = ssr::client_context(&state);

    let (token, profile) = state
        .auth
        .sign_in(credentials, client)
        .await
        .map_err(from_domain)?;

    ssr::set_session_cookie(&state, &token)?;
    tracing::info!(user_id = %profile.id, "signed in");

    leptos_axum::redirect("/");
    Ok(())
}

/// End the current session.
///
/// Idempotent, and it clears the cookie even when no session was found, so a
/// stale cookie cannot leave the browser stuck in a signed-in-looking state.
#[server(endpoint = "sign_out")]
pub async fn sign_out() -> Result<(), AppError> {
    use crate::error::from_domain;
    use crate::server::ssr;

    let state = ssr::state()?;

    if let Some(token) = ssr::session_token(&state) {
        state.auth.sign_out(&token).await.map_err(from_domain)?;
    }

    ssr::clear_session_cookie(&state)?;
    leptos_axum::redirect("/signin");
    Ok(())
}

/// The signed-in user, or `None` for an anonymous visitor.
///
/// `None` rather than an error: not being signed in is the ordinary state of a
/// visitor, and the shell renders differently for it rather than treating it as
/// a failure.
#[server(endpoint = "current_user")]
pub async fn current_user() -> Result<Option<UserProfile>, AppError> {
    use app_core::ApiError;

    use crate::server::ssr;

    let state = ssr::state()?;

    match ssr::require_user(&state).await {
        Ok(user) => Ok(Some(user.profile)),
        Err(e) if e.as_api() == Some(&ApiError::Unauthenticated) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Every active session for the signed-in user.
#[server(endpoint = "list_sessions")]
pub async fn list_sessions() -> Result<Vec<SessionSummary>, AppError> {
    use crate::error::from_domain;
    use crate::server::ssr;

    let state = ssr::state()?;
    let user = ssr::require_user(&state).await?;

    state.auth.list_sessions(&user).await.map_err(from_domain)
}

/// Revoke one of the signed-in user's sessions.
///
/// Revoking the current session is allowed and signs the user out here as well;
/// the cookie is cleared so the browser does not keep presenting a token that
/// no longer resolves.
#[server(endpoint = "revoke_session")]
pub async fn revoke_session(session_id: Uuid) -> Result<(), AppError> {
    use crate::error::from_domain;
    use crate::server::ssr;

    let state = ssr::state()?;
    let user = ssr::require_user(&state).await?;

    state
        .auth
        .revoke_session(&user, session_id)
        .await
        .map_err(from_domain)?;

    tracing::info!(user_id = %user.profile.id, %session_id, "session revoked");

    if session_id == user.session_id {
        ssr::clear_session_cookie(&state)?;
        leptos_axum::redirect("/signin");
    }

    Ok(())
}
