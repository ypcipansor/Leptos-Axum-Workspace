//! Server-side domain logic: configuration, persistence, and authentication.
//!
//! This crate holds everything the application does once a request has been
//! decoded, and nothing about how requests arrive. It has no dependency on
//! Leptos or Axum routing, which keeps two things true:
//!
//! 1. `#[server]` functions in the UI crate can call it without creating a
//!    circular dependency with the binary crate.
//! 2. The UI framework can be replaced without rewriting the backend.
//!
//! Layering, outermost first:
//!
//! - [`auth`] -- use cases (register, sign in, revoke). The only layer callers
//!   should reach for.
//! - [`repo`] -- SQL. Every query lives here; nothing above writes SQL.
//! - [`db`] / [`config`] -- process-level infrastructure.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod repo;
pub mod state;

pub use config::{Config, ConfigError, Environment};
pub use error::DomainError;
pub use state::AppState;

/// Name reported by the health endpoint and used as the tracing service name.
pub const SERVICE_NAME: &str = "app-server";

/// Version reported by the health endpoint, taken from the crate metadata.
pub const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");
