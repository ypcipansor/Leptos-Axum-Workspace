use std::sync::LazyLock;

use app_core::Password;
use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use base64ct::{Base64UrlUnpadded, Encoding};

use crate::error::DomainError;

/// Salt width in bytes. 128 bits is the value RFC 9106 recommends.
const SALT_BYTES: usize = 16;

/// A password hash in PHC string format, ready to be stored.
pub type PasswordHashString = String;

/// Hash a password with Argon2id.
///
/// Argon2id replaces the previous bcrypt implementation. bcrypt silently
/// truncates input at 72 bytes -- which the old code worked around by capping
/// passwords at 72 characters -- and is memory-cheap enough that GPU attacks
/// remain practical. Argon2id is the algorithm OWASP recommends for new
/// applications.
///
/// The default parameters (19 MiB, 2 iterations, 1 lane) are the OWASP
/// baseline. They are embedded in the resulting PHC string, so raising them
/// later does not invalidate existing hashes.
///
/// Runs on the blocking pool: hashing deliberately costs tens of milliseconds
/// of CPU, which would otherwise stall every other future on the worker thread.
pub async fn hash(password: Password) -> Result<PasswordHashString, DomainError> {
    tokio::task::spawn_blocking(move || hash_blocking(password.expose())).await?
}

/// Verify a password against a stored PHC hash.
///
/// Returns `Ok(false)` for a mismatch and reserves `Err` for genuine failures,
/// so a malformed stored hash is never silently treated as a wrong password.
pub async fn verify(password: Password, hash: PasswordHashString) -> Result<bool, DomainError> {
    tokio::task::spawn_blocking(move || verify_blocking(password.expose(), &hash)).await?
}

/// Spend the same CPU as a real verification, then fail.
///
/// Called when the username does not exist. Without it, a missing user returns
/// in microseconds while a wrong password takes tens of milliseconds, and that
/// difference lets an attacker enumerate valid usernames.
///
/// The reference hash is computed once from a fixed passphrase rather than
/// pasted in as a literal, as the previous implementation did with a hard-coded
/// bcrypt string. A literal cannot follow changes to the algorithm or its
/// parameters, so the timing it equalises drifts out of date silently.
pub async fn verify_dummy(password: Password) -> Result<(), DomainError> {
    let reference = dummy_hash()?.clone();
    let _ = verify(password, reference).await?;
    Ok(())
}

static DUMMY_HASH: LazyLock<Result<PasswordHashString, String>> = LazyLock::new(|| {
    // Drawn from the CSPRNG at startup rather than written as a literal. The
    // value is irrelevant -- nothing ever verifies against it successfully, and
    // it is discarded after the first hash -- but generating it means the
    // binary carries no constant that reads as a credential, to a reviewer or
    // to a static analyser.
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;

    hash_blocking(&Base64UrlUnpadded::encode_string(&bytes)).map_err(|e| e.to_string())
});

fn dummy_hash() -> Result<&'static PasswordHashString, DomainError> {
    DUMMY_HASH
        .as_ref()
        .map_err(|e| DomainError::PasswordHash(e.clone()))
}

fn hash_blocking(password: &str) -> Result<PasswordHashString, DomainError> {
    let mut salt_bytes = [0_u8; SALT_BYTES];
    getrandom::fill(&mut salt_bytes)
        .map_err(|e| DomainError::PasswordHash(format!("secure randomness unavailable: {e}")))?;

    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| DomainError::PasswordHash(format!("salt encoding failed: {e}")))?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| DomainError::PasswordHash(e.to_string()))
}

fn verify_blocking(password: &str, hash: &str) -> Result<bool, DomainError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| DomainError::PasswordHash(format!("stored hash is malformed: {e}")))?;

    match Argon2::default().verify_password(password.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(DomainError::PasswordHash(e.to_string())),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn password(raw: &str) -> Password {
        Password::parse(raw).unwrap()
    }

    #[tokio::test]
    async fn hashing_then_verifying_succeeds() {
        let stored = hash(password("correct horse battery")).await.unwrap();
        assert!(
            verify(password("correct horse battery"), stored)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn verification_rejects_the_wrong_password() {
        let stored = hash(password("correct horse battery")).await.unwrap();
        assert!(
            !verify(password("incorrect horse battery"), stored)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn hash_is_argon2id_and_carries_its_parameters() {
        let stored = hash(password("correct horse battery")).await.unwrap();
        assert!(stored.starts_with("$argon2id$"), "got {stored}");
        assert!(stored.contains("m="), "parameters missing from {stored}");
    }

    #[tokio::test]
    async fn same_password_hashes_differently_each_time() {
        // A per-hash random salt is what stops identical passwords from being
        // identifiable across accounts in a leaked dump.
        let first = hash(password("correct horse battery")).await.unwrap();
        let second = hash(password("correct horse battery")).await.unwrap();
        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn stored_hash_does_not_contain_the_password() {
        let stored = hash(password("correct horse battery")).await.unwrap();
        assert!(!stored.contains("correct"));
        assert!(!stored.contains("horse"));
    }

    #[tokio::test]
    async fn long_passwords_are_not_truncated() {
        // The distinguishing failure of bcrypt: it ignores everything past 72
        // bytes, so these two passwords would have verified interchangeably.
        let base = "a".repeat(80);
        let stored = hash(password(&base)).await.unwrap();

        let mut differing_past_72 = "a".repeat(79);
        differing_past_72.push('b');

        assert!(!verify(password(&differing_past_72), stored).await.unwrap());
    }

    #[tokio::test]
    async fn malformed_stored_hash_is_an_error_not_a_mismatch() {
        let result = verify(
            password("correct horse battery"),
            "not-a-phc-string".to_owned(),
        )
        .await;
        assert!(matches!(result, Err(DomainError::PasswordHash(_))));
    }

    #[tokio::test]
    async fn dummy_verification_completes_without_revealing_a_match() {
        assert!(verify_dummy(password("any password here")).await.is_ok());
    }

    #[tokio::test]
    async fn dummy_reference_is_a_valid_argon2id_hash() {
        // If this drifted out of shape, the timing it equalises would be wrong.
        assert!(dummy_hash().unwrap().starts_with("$argon2id$"));
    }
}
