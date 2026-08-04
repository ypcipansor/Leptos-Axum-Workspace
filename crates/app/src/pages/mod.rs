//! Route targets.

// See the note in `components::mod` on why these are public.
pub mod dashboard;
pub mod not_found;
pub mod sign_in;
pub mod sign_up;

pub use dashboard::DashboardPage;
pub use not_found::NotFoundPage;
pub use sign_in::SignInPage;
pub use sign_up::SignUpPage;
