//! The HTTP layer: routing, middleware, health probes and background tasks.
//!
//! Exposed as a library so integration tests can build the very same router the
//! binary serves, rather than testing an approximation of it. The previous test
//! suite shelled out to `cargo run -p backend &` and hoped the process came up
//! in time.

pub mod health;
pub mod router;
pub mod shutdown;
pub mod tasks;
pub mod telemetry;

pub use router::{ServerState, build};
