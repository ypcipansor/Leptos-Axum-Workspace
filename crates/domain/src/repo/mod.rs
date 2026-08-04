//! Every SQL statement in the application.
//!
//! Nothing above this layer writes SQL, and nothing in this layer makes policy
//! decisions -- it translates between rows and Rust values, nothing more.
//!
//! All queries use the `sqlx::query_as!` family, which checks them against the
//! real schema at compile time. A typo in a column name, a wrong nullability,
//! or a type that does not line up is a build failure rather than a runtime
//! surprise. The previous implementation used the unchecked `sqlx::query()` API
//! with stringly-typed `row.get("column")` access, which caught none of that.
//!
//! Compilation reads the schema from either a live `DATABASE_URL` or the
//! committed `.sqlx/` cache (`SQLX_OFFLINE=true`), which is why that directory
//! is checked in and refreshed by `just prepare`.

pub mod sessions;
pub mod users;
