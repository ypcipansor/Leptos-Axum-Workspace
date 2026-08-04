//! Authentication: password hashing, session tokens, and the use cases built
//! on them.
//!
//! The session model is server-side and opaque. A sign-in mints 256 bits of
//! CSPRNG output, hands it to the browser in an `HttpOnly` cookie, and stores
//! only its SHA-256 digest. Authenticating a request means hashing the
//! presented token and looking that digest up.
//!
//! Two properties follow, and both were missing before:
//!
//! - A database dump yields no usable credential, because the token itself is
//!   never written down.
//! - Revocation is immediate. Deleting the row ends the session on the next
//!   request, with no window in which an already-issued credential stays valid.

pub mod client;
pub mod cookie;
pub mod password;
pub mod rate_limit;
pub mod token;

mod service;

pub use crate::repo::sessions::ClientContext;
pub use client::resolve_context;
pub use service::{AuthService, CurrentUser};
pub use token::SessionToken;
