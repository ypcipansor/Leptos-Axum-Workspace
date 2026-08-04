//! Contracts shared by the Leptos UI and the Axum server.
//!
//! Everything the two halves of the application agree on lives here: the
//! validated newtypes, the wire DTOs, and the error enum the UI renders.
//!
//! The point of this crate is that validation cannot drift. Previously the
//! username and password rules were written twice -- once in the browser and
//! once in the request handler -- and nothing kept them in sync. Here the rules
//! exist once, inside [`Username::parse`] and [`Password::parse`], and both
//! sides are structurally unable to construct an invalid value.
//!
//! This crate compiles to wasm as well as native, so it must stay free of
//! filesystem, clock and randomness dependencies.

mod error;
mod health;
mod session;
mod user;
mod validation;

pub use error::ApiError;
pub use health::{HealthStatus, ServiceState};
pub use session::SessionSummary;
pub use user::{
    Credentials, PASSWORD_MAX_LEN, PASSWORD_MIN_LEN, Password, USERNAME_MAX_LEN, USERNAME_MIN_LEN,
    UserProfile, Username,
};
pub use validation::ValidationError;

/// Human-readable application name, rendered in the UI shell and reported by
/// the health endpoint.
pub const APP_NAME: &str = "Leptos Axum Workspace";
