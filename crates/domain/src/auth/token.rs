use core::fmt;

use base64ct::{Base64UrlUnpadded, Encoding};
use sha2::{Digest, Sha256};

use crate::error::DomainError;

/// Bytes of entropy in a session token.
///
/// 256 bits, drawn from the OS CSPRNG. The previous implementation used a
/// `UUIDv4`, which carries 122 bits and advertises its structure; there is no
/// reason to hand an attacker either constraint.
const TOKEN_BYTES: usize = 32;

/// Length of the stored digest. Mirrored by a CHECK constraint on the column.
pub const TOKEN_HASH_LEN: usize = 32;

/// A session token in its transportable form.
///
/// This is the value carried by the session cookie and it is the only secret
/// that authenticates a request. It is never stored: the database holds
/// [`TokenHash`] instead, so a leaked dump yields nothing presentable.
///
/// `Debug` is redacted and `Display` is not implemented, so the token cannot
/// reach a log line by accident. Reading it requires [`SessionToken::expose`].
#[derive(Clone, PartialEq, Eq)]
pub struct SessionToken(String);

impl SessionToken {
    /// Draw a fresh token from the operating system's CSPRNG.
    pub fn generate() -> Result<Self, DomainError> {
        let mut bytes = [0_u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes).map_err(|e| {
            DomainError::PasswordHash(format!("secure randomness unavailable: {e}"))
        })?;

        Ok(Self(Base64UrlUnpadded::encode_string(&bytes)))
    }

    /// Accept a token presented by a client. Performs no validation beyond
    /// shape; authority comes from finding its hash in the database.
    #[must_use]
    pub fn from_cookie_value(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        // Cheap sanity check so obviously-wrong input never reaches the
        // database. A base64url-encoded 32-byte value is always 43 characters.
        if trimmed.len() != 43 || !trimmed.bytes().all(is_base64url_byte) {
            return None;
        }
        Some(Self(trimmed.to_owned()))
    }

    /// Borrow the secret. Named to make every call site conspicuous in review.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Derive the value stored in the database.
    ///
    /// SHA-256 is the right primitive here, rather than a password hash: the
    /// input already has 256 bits of entropy, so there is nothing to brute
    /// force, and lookups happen on every authenticated request.
    #[must_use]
    pub fn hash(&self) -> TokenHash {
        let digest = Sha256::digest(self.0.as_bytes());
        let mut out = [0_u8; TOKEN_HASH_LEN];
        out.copy_from_slice(&digest);
        TokenHash(out)
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionToken(<redacted>)")
    }
}

/// The SHA-256 digest of a [`SessionToken`], as stored in `sessions.token_hash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenHash([u8; TOKEN_HASH_LEN]);

impl TokenHash {
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

const fn is_base64url_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generated_tokens_are_unique() {
        let tokens: HashSet<String> = (0..1_000)
            .map(|_| SessionToken::generate().unwrap().expose().to_owned())
            .collect();
        assert_eq!(tokens.len(), 1_000, "generator produced a collision");
    }

    #[test]
    fn generated_tokens_are_url_safe_and_full_length() {
        let token = SessionToken::generate().unwrap();
        let value = token.expose();
        assert_eq!(value.len(), 43, "expected 32 bytes base64url-encoded");
        assert!(
            value.bytes().all(is_base64url_byte),
            "not url-safe: {value}"
        );
    }

    #[test]
    fn hashing_is_deterministic() {
        let token = SessionToken::generate().unwrap();
        assert_eq!(token.hash(), token.hash());
    }

    #[test]
    fn different_tokens_hash_differently() {
        let a = SessionToken::generate().unwrap();
        let b = SessionToken::generate().unwrap();
        assert_ne!(a.hash(), b.hash());
    }

    #[test]
    fn hash_is_the_declared_width() {
        // The sessions table enforces octet_length(token_hash) = 32; a mismatch
        // here would surface as a constraint violation at insert time.
        let token = SessionToken::generate().unwrap();
        assert_eq!(token.hash().as_bytes().len(), TOKEN_HASH_LEN);
    }

    #[test]
    fn hash_does_not_contain_the_token() {
        let token = SessionToken::generate().unwrap();
        let hash_bytes = token.hash();
        assert!(!hash_bytes.as_bytes().starts_with(token.expose().as_bytes()));
    }

    #[test]
    fn debug_output_is_redacted() {
        let token = SessionToken::generate().unwrap();
        let rendered = format!("{token:?}");
        assert_eq!(rendered, "SessionToken(<redacted>)");
        assert!(!rendered.contains(token.expose()));
    }

    #[test]
    fn cookie_values_round_trip() {
        let token = SessionToken::generate().unwrap();
        let parsed = SessionToken::from_cookie_value(token.expose()).unwrap();
        assert_eq!(parsed, token);
        assert_eq!(parsed.hash(), token.hash());
    }

    #[test]
    fn malformed_cookie_values_are_rejected_before_reaching_the_database() {
        assert!(SessionToken::from_cookie_value("").is_none());
        assert!(SessionToken::from_cookie_value("too-short").is_none());
        assert!(SessionToken::from_cookie_value(&"a".repeat(44)).is_none());
        // Padding and non-url-safe alphabet characters.
        assert!(SessionToken::from_cookie_value(&format!("{}=", "a".repeat(42))).is_none());
        assert!(SessionToken::from_cookie_value(&format!("{}+", "a".repeat(42))).is_none());
        // A classic injection probe must not survive the shape check.
        assert!(SessionToken::from_cookie_value("' OR 1=1 --").is_none());
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        let token = SessionToken::generate().unwrap();
        let padded = format!("  {}  ", token.expose());
        assert_eq!(SessionToken::from_cookie_value(&padded).unwrap(), token);
    }
}
