use core::fmt;
use core::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::validation::ValidationError;

/// Shortest accepted username.
pub const USERNAME_MIN_LEN: usize = 3;
/// Longest accepted username.
pub const USERNAME_MAX_LEN: usize = 32;

/// Shortest accepted password.
///
/// Twelve characters, not the four the previous implementation allowed. Argon2
/// imposes no upper bound of its own (unlike bcrypt's 72-byte truncation), so
/// the ceiling exists purely to bound hashing work per request.
pub const PASSWORD_MIN_LEN: usize = 12;
/// Longest accepted password.
pub const PASSWORD_MAX_LEN: usize = 128;

/// A username that has been checked against the rules in [`Username::parse`].
///
/// Deserialization goes through the same parser, so a `Username` received over
/// the network is as trustworthy as one built locally -- there is no way to
/// construct an invalid value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Username(String);

impl Username {
    /// Validate and normalize a username.
    ///
    /// Surrounding whitespace is trimmed. Case is preserved for display; the
    /// database enforces uniqueness case-insensitively via a `lower(username)`
    /// index, so `Alice` and `alice` cannot both be registered.
    pub fn parse(raw: &str) -> Result<Self, ValidationError> {
        let trimmed = raw.trim();
        let len = trimmed.chars().count();

        if len < USERNAME_MIN_LEN {
            return Err(ValidationError::UsernameTooShort {
                min: USERNAME_MIN_LEN,
                actual: len,
            });
        }
        if len > USERNAME_MAX_LEN {
            return Err(ValidationError::UsernameTooLong {
                max: USERNAME_MAX_LEN,
                actual: len,
            });
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        {
            return Err(ValidationError::UsernameInvalidCharacters);
        }
        // Leading punctuation invites homograph-style confusion between
        // accounts such as `.admin` and `admin`.
        if !trimmed.starts_with(|c: char| c.is_ascii_alphanumeric()) {
            return Err(ValidationError::UsernameInvalidStart);
        }

        Ok(Self(trimmed.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for Username {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Username {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.0
    }
}

/// A password that has been length-checked but never hashed.
///
/// The inner value is deliberately unreachable outside [`Password::expose`], and
/// `Debug` is redacted so a stray `tracing::debug!` or panic message cannot leak
/// a credential into the logs.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Password(String);

impl Password {
    pub fn parse(raw: &str) -> Result<Self, ValidationError> {
        let len = raw.chars().count();

        if len < PASSWORD_MIN_LEN {
            return Err(ValidationError::PasswordTooShort {
                min: PASSWORD_MIN_LEN,
                actual: len,
            });
        }
        if len > PASSWORD_MAX_LEN {
            return Err(ValidationError::PasswordTooLong {
                max: PASSWORD_MAX_LEN,
                actual: len,
            });
        }

        // Note: no trimming. Leading and trailing spaces are legitimate
        // password characters and silently removing them would lock users out
        // of accounts created elsewhere.
        Ok(Self(raw.to_owned()))
    }

    /// Borrow the secret. Named to make every call site conspicuous in review.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}

impl FromStr for Password {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Password {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

/// Sign-in / sign-up input. Both fields are already validated by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub username: Username,
    pub password: Password,
}

/// The authenticated user, as returned to the browser.
///
/// Contains no password material, by construction rather than by convention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: Username,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn username_accepts_valid_input() {
        assert_eq!(Username::parse("alice").unwrap().as_str(), "alice");
        assert_eq!(Username::parse("a1_b-c.d").unwrap().as_str(), "a1_b-c.d");
    }

    #[test]
    fn username_trims_surrounding_whitespace() {
        assert_eq!(Username::parse("  alice  ").unwrap().as_str(), "alice");
    }

    #[test]
    fn username_preserves_case_for_display() {
        assert_eq!(Username::parse("Alice").unwrap().as_str(), "Alice");
    }

    #[test]
    fn username_rejects_out_of_range_lengths() {
        assert_eq!(
            Username::parse("ab"),
            Err(ValidationError::UsernameTooShort { min: 3, actual: 2 })
        );
        let long = "a".repeat(USERNAME_MAX_LEN + 1);
        assert_eq!(
            Username::parse(&long),
            Err(ValidationError::UsernameTooLong {
                max: USERNAME_MAX_LEN,
                actual: USERNAME_MAX_LEN + 1,
            })
        );
    }

    #[test]
    fn username_rejects_disallowed_characters() {
        assert_eq!(
            Username::parse("alice bob"),
            Err(ValidationError::UsernameInvalidCharacters)
        );
        assert_eq!(
            Username::parse("alice@example.com"),
            Err(ValidationError::UsernameInvalidCharacters)
        );
    }

    #[test]
    fn username_rejects_punctuation_prefix() {
        assert_eq!(
            Username::parse(".admin"),
            Err(ValidationError::UsernameInvalidStart)
        );
        assert_eq!(
            Username::parse("_admin"),
            Err(ValidationError::UsernameInvalidStart)
        );
    }

    #[test]
    fn username_length_counts_characters_not_bytes() {
        // Four multi-byte characters is over the byte minimum but the rule is
        // expressed in characters, and these are not ASCII alphanumeric.
        assert_eq!(
            Username::parse("日本語です"),
            Err(ValidationError::UsernameInvalidCharacters)
        );
    }

    #[test]
    fn password_enforces_bounds() {
        assert!(Password::parse("correct horse battery").is_ok());
        assert_eq!(
            Password::parse("short"),
            Err(ValidationError::PasswordTooShort { min: 12, actual: 5 })
        );
        let long = "a".repeat(PASSWORD_MAX_LEN + 1);
        assert_eq!(
            Password::parse(&long),
            Err(ValidationError::PasswordTooLong {
                max: PASSWORD_MAX_LEN,
                actual: PASSWORD_MAX_LEN + 1,
            })
        );
    }

    #[test]
    fn password_preserves_surrounding_whitespace() {
        let pw = Password::parse("  spaced out password  ").unwrap();
        assert_eq!(pw.expose(), "  spaced out password  ");
    }

    #[test]
    fn password_debug_output_is_redacted() {
        let pw = Password::parse("super secret value").unwrap();
        let rendered = format!("{pw:?}");
        assert_eq!(rendered, "Password(<redacted>)");
        assert!(!rendered.contains("super"));
    }

    #[test]
    fn credentials_debug_does_not_leak_the_password() {
        let creds = Credentials {
            username: Username::parse("alice").unwrap(),
            password: Password::parse("super secret value").unwrap(),
        };
        assert!(!format!("{creds:?}").contains("super secret"));
    }

    #[test]
    fn username_deserialization_runs_the_validator() {
        // The whole point of `try_from`: an invalid username cannot enter the
        // process, even from an untrusted wire payload.
        assert!(serde_json::from_str::<Username>("\"ok_name\"").is_ok());
        assert!(serde_json::from_str::<Username>("\"a\"").is_err());
        assert!(serde_json::from_str::<Username>("\"has space\"").is_err());
    }

    #[test]
    fn password_deserialization_runs_the_validator() {
        assert!(serde_json::from_str::<Password>("\"long enough password\"").is_ok());
        assert!(serde_json::from_str::<Password>("\"tiny\"").is_err());
    }

    #[test]
    fn credentials_round_trip_through_json() {
        let creds = Credentials {
            username: Username::parse("alice").unwrap(),
            password: Password::parse("long enough password").unwrap(),
        };
        let json = serde_json::to_string(&creds).unwrap();
        let back: Credentials = serde_json::from_str(&json).unwrap();
        assert_eq!(creds, back);
    }

    #[test]
    fn user_profile_round_trips_through_json() {
        let profile = UserProfile {
            id: Uuid::nil(),
            username: Username::parse("alice").unwrap(),
            created_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert_eq!(serde_json::from_str::<UserProfile>(&json).unwrap(), profile);
    }
}
