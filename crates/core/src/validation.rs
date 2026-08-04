use serde::{Deserialize, Serialize};

/// Why a value failed to parse into one of the validated newtypes.
///
/// This crosses the network: the server returns it, the browser renders it. Each
/// variant carries the bound it violated so the UI can show a specific message
/// without re-deriving the rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValidationError {
    #[error("Username must be at least {min} characters (got {actual}).")]
    UsernameTooShort { min: usize, actual: usize },

    #[error("Username must be at most {max} characters (got {actual}).")]
    UsernameTooLong { max: usize, actual: usize },

    #[error("Username may only contain letters, digits, '.', '-' and '_'.")]
    UsernameInvalidCharacters,

    #[error("Username must start with a letter or digit.")]
    UsernameInvalidStart,

    #[error("Password must be at least {min} characters (got {actual}).")]
    PasswordTooShort { min: usize, actual: usize },

    #[error("Password must be at most {max} characters (got {actual}).")]
    PasswordTooLong { max: usize, actual: usize },
}
