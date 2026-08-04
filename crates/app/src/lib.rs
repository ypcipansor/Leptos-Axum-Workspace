//! The user interface, compiled twice.
//!
//! Under the `ssr` feature this crate renders HTML on the server and its
//! `#[server]` function bodies talk to the database. Under `hydrate` the very
//! same components compile to wasm and take over the already-rendered DOM in
//! the browser, while the server function bodies are replaced by typed network
//! calls.
//!
//! One definition of every component, one definition of every endpoint. That is
//! the property the previous split -- a client-only SPA calling a hand-written
//! REST API -- could not have.

// Leptos encodes an entire view tree in the type system, so a nested view --
// a table inside a suspense boundary inside a transition -- produces a type
// hundreds of levels deep. The hydration build exceeds the default limit of 128
// and fails with "queries overflow the depth limit". This costs nothing at
// runtime; it only lets the compiler finish resolving those types.
#![recursion_limit = "512"]

pub mod app;
pub mod components;
pub mod error;
pub mod pages;
pub mod server;

pub use app::{App, shell};
pub use error::AppError;
