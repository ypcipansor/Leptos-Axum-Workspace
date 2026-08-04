//! Reusable UI primitives.
//!
//! This module previously existed as a single doc comment promising components
//! that were never written, while every page hand-rolled its own markup. These
//! are the real thing: each one owns its accessibility behaviour so a page
//! cannot forget it.

// Public rather than private-with-re-exports: the `#[component]` macro expands
// each of these into a function plus a props struct, and only the struct is
// nameable through a `pub use`. Leaving the modules private makes the generated
// function unreachable, which `unreachable_pub` correctly flags.
pub mod alert;
pub mod button;
pub mod card;
pub mod spinner;
pub mod text_field;
pub mod theme;

pub use alert::{Alert, AlertKind};
pub use button::{Button, ButtonKind};
pub use card::Card;
pub use spinner::Spinner;
pub use text_field::TextField;
pub use theme::{THEME_INIT_SCRIPT, ThemeToggle};
