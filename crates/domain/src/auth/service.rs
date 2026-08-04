use core::time::Duration;
use std::sync::Arc;

use app_core::{Credentials, SessionSummary, UserProfile, Username};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::password;
use crate::auth::rate_limit::{self, RateLimiter};
use crate::auth::token::SessionToken;
use crate::error::DomainError;
use crate::repo::sessions::ClientContext;
use crate::repo::{sessions, users};

/// The identity behind an authenticated request.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub profile: UserProfile,
    /// Which session authenticated this request. Used to mark the current row
    /// in the session list.
    pub session_id: Uuid,
}

/// Authentication use cases.
///
/// Everything a caller needs to register, sign in, inspect and revoke sessions.
/// Callers never touch [`crate::repo`] directly, so the invariants enforced
/// here -- timing equalisation, ownership checks, expiry -- cannot be bypassed
/// by reaching around them.
#[derive(Debug, Clone)]
pub struct AuthService {
    pool: PgPool,
    session_ttl: Duration,
    /// Shared across every clone of the service, so the budget is process-wide
    /// rather than per-request.
    rate_limiter: Arc<RateLimiter>,
}

impl AuthService {
    #[must_use]
    pub fn new(pool: PgPool, session_ttl: Duration) -> Self {
        Self {
            pool,
            session_ttl,
            rate_limiter: Arc::new(RateLimiter::default()),
        }
    }

    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Create an account.
    ///
    /// The password is hashed before the insert is attempted, so a name
    /// collision costs the same wall-clock time as a success. Doing it the
    /// other way round would let an attacker distinguish taken names by
    /// response time alone.
    pub async fn register(&self, credentials: Credentials) -> Result<UserProfile, DomainError> {
        let password_hash = password::hash(credentials.password).await?;
        let record =
            users::insert(&self.pool, credentials.username.as_str(), &password_hash).await?;

        Ok(UserProfile {
            id: record.id,
            username: parse_stored_username(&record.username)?,
            created_at: record.created_at,
        })
    }

    /// Verify credentials and mint a session.
    ///
    /// Returns the raw token, which the transport layer must place in an
    /// `HttpOnly` cookie and never expose to JavaScript.
    pub async fn sign_in(
        &self,
        credentials: Credentials,
        client: ClientContext,
    ) -> Result<(SessionToken, UserProfile), DomainError> {
        // Checked before any database or hashing work, so a throttled caller
        // costs almost nothing to reject.
        let attempt_key = rate_limit::attempt_key(client.ip_address, credentials.username.as_str());
        self.rate_limiter.check(&attempt_key)?;

        let Some(record) =
            users::find_by_username(&self.pool, credentials.username.as_str()).await?
        else {
            // Burn the same CPU a real verification would, then fail. Skipping
            // this makes "no such user" measurably faster than "wrong
            // password", which is enough to enumerate valid usernames.
            password::verify_dummy(credentials.password).await?;
            return Err(DomainError::InvalidCredentials);
        };

        if !password::verify(credentials.password, record.password_hash).await? {
            return Err(DomainError::InvalidCredentials);
        }

        // Only failed attempts count toward the limit, so signing in from
        // several devices never walks an ordinary user into a lockout.
        self.rate_limiter.record_success(&attempt_key);

        let token = SessionToken::generate()?;
        sessions::insert(
            &self.pool,
            record.id,
            &token.hash(),
            &client,
            self.session_ttl.as_secs_f64(),
        )
        .await?;

        let profile = UserProfile {
            id: record.id,
            username: parse_stored_username(&record.username)?,
            created_at: record.created_at,
        };

        Ok((token, profile))
    }

    /// Resolve a session token to the user it authenticates.
    ///
    /// Also slides the expiry window forward, throttled so an idle-but-open tab
    /// does not write to the sessions table on every poll.
    pub async fn authenticate(&self, token: &SessionToken) -> Result<CurrentUser, DomainError> {
        let session = sessions::find_active_by_token_hash(&self.pool, &token.hash())
            .await?
            .ok_or(DomainError::Unauthenticated)?;

        sessions::touch(
            &self.pool,
            session.session_id,
            self.session_ttl.as_secs_f64(),
            self.touch_throttle().as_secs_f64(),
        )
        .await?;

        Ok(CurrentUser {
            profile: UserProfile {
                id: session.user_id,
                username: parse_stored_username(&session.username)?,
                created_at: session.user_created_at,
            },
            session_id: session.session_id,
        })
    }

    /// End the session behind `token`.
    ///
    /// Idempotent: signing out twice, or with a token that was already revoked
    /// elsewhere, is a success. The caller's goal -- that this token no longer
    /// authenticates anything -- holds either way.
    pub async fn sign_out(&self, token: &SessionToken) -> Result<(), DomainError> {
        sessions::delete_by_token_hash(&self.pool, &token.hash()).await?;
        Ok(())
    }

    /// Every active session for the current user, newest first.
    pub async fn list_sessions(
        &self,
        current: &CurrentUser,
    ) -> Result<Vec<SessionSummary>, DomainError> {
        let records = sessions::list_active_for_user(&self.pool, current.profile.id).await?;

        Ok(records
            .into_iter()
            .map(|record| SessionSummary {
                is_current: record.id == current.session_id,
                id: record.id,
                user_agent: record.user_agent,
                ip_address: record.ip_address.map(|ip| ip.to_string()),
                created_at: record.created_at,
                last_seen_at: record.last_seen_at,
                expires_at: record.expires_at,
            })
            .collect())
    }

    /// Revoke one session belonging to the current user.
    ///
    /// Ownership is enforced inside the `DELETE`, and a session belonging to
    /// someone else is reported as [`DomainError::SessionNotFound`] -- the same
    /// answer as an id that never existed, so this cannot be used to probe for
    /// other users' sessions.
    pub async fn revoke_session(
        &self,
        current: &CurrentUser,
        session_id: Uuid,
    ) -> Result<(), DomainError> {
        let removed = sessions::delete_for_user(&self.pool, session_id, current.profile.id).await?;

        if removed == 0 {
            return Err(DomainError::SessionNotFound);
        }
        Ok(())
    }

    /// Revoke every session for the current user, including this one.
    pub async fn revoke_all_sessions(&self, current: &CurrentUser) -> Result<u64, DomainError> {
        sessions::delete_all_for_user(&self.pool, current.profile.id).await
    }

    /// Delete expired rows. Driven by the background sweep in the server crate.
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, DomainError> {
        sessions::delete_expired(&self.pool).await
    }

    fn touch_throttle(&self) -> Duration {
        touch_throttle(self.session_ttl)
    }
}

/// How stale `last_seen_at` must be before a request rewrites it.
///
/// One twentieth of the session lifetime, so the sliding window stays accurate
/// to within 5% while collapsing bursts of requests into at most one write.
///
/// A free function rather than a method so it can be tested without standing up
/// a connection pool.
fn touch_throttle(session_ttl: Duration) -> Duration {
    session_ttl / 20
}

/// Re-parse a username loaded from the database.
///
/// The column has CHECK constraints mirroring [`Username::parse`], so failure
/// means the table holds something those constraints should have rejected. That
/// is a data integrity fault, not a user error, and it is reported as one
/// rather than being smuggled back as a validation message.
fn parse_stored_username(raw: &str) -> Result<Username, DomainError> {
    Username::parse(raw).map_err(|e| {
        DomainError::DataIntegrity(format!("stored username `{raw}` failed validation: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_throttle_is_a_small_fraction_of_the_lifetime() {
        let week = Duration::from_hours(168);
        assert_eq!(touch_throttle(week), Duration::from_mins(504));

        let minute = Duration::from_mins(1);
        assert_eq!(touch_throttle(minute), Duration::from_secs(3));
    }

    #[test]
    fn touch_throttle_stays_below_the_lifetime_it_protects() {
        // A throttle at or above the TTL would stop the window from ever
        // sliding, expiring sessions that are actively in use.
        for secs in [1, 20, 60, 3_600, 604_800] {
            let ttl = Duration::from_secs(secs);
            assert!(
                touch_throttle(ttl) < ttl,
                "throttle did not shrink for {secs}s"
            );
        }
    }

    #[test]
    fn stored_username_round_trips() {
        let parsed = parse_stored_username("alice").expect("valid stored username");
        assert_eq!(parsed.as_str(), "alice");
    }

    #[test]
    fn corrupt_stored_username_is_an_integrity_fault_not_a_validation_message() {
        // A validation error here would be shown to the user as though they had
        // typed something wrong, when in fact the database is inconsistent.
        let error = parse_stored_username("!!").expect_err("should reject");
        assert!(matches!(error, DomainError::DataIntegrity(_)));
        assert!(error.is_internal());
    }
}
