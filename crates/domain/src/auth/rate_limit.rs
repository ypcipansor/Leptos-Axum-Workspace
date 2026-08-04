use core::time::Duration;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::error::DomainError;

/// Attempts allowed per key within [`WINDOW`].
pub const MAX_ATTEMPTS: u32 = 10;

/// Length of the counting window.
pub const WINDOW: Duration = Duration::from_mins(5);

/// Entries retained before a sweep is triggered.
const SWEEP_THRESHOLD: usize = 10_000;

/// A fixed-window rate limiter for authentication attempts.
///
/// The previous implementation had no throttling at all: `/login` accepted
/// unlimited guesses at whatever rate a client could manage.
///
/// Limiting here rather than with an HTTP layer is deliberate. A blanket
/// middleware can only see the request's address, so it either throttles all
/// traffic or nothing, and a distributed attempt against one account looks like
/// ordinary traffic from many addresses. Keying on the *username* as well means
/// a single account cannot be ground down from a botnet, while a shared office
/// address does not lock out everyone behind it.
///
/// A fixed window admits up to twice the limit across a window boundary. For
/// slowing credential stuffing that is immaterial -- 20 guesses per 5 minutes
/// is as useless to an attacker as 10 -- and it costs one integer per key
/// instead of a timestamp log.
#[derive(Debug)]
pub struct RateLimiter {
    windows: Mutex<HashMap<String, Window>>,
    max_attempts: u32,
    window: Duration,
}

#[derive(Debug, Clone, Copy)]
struct Window {
    started: Instant,
    count: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(MAX_ATTEMPTS, WINDOW)
    }
}

impl RateLimiter {
    #[must_use]
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
            max_attempts,
            window,
        }
    }

    /// Record an attempt against `key`, rejecting it if the budget is spent.
    pub fn check(&self, key: &str) -> Result<(), DomainError> {
        self.check_at(key, Instant::now())
    }

    /// Release one unit of budget.
    ///
    /// Called after a *successful* sign-in so that ordinary use -- a person
    /// signing in on several devices, a shared address in an office -- never
    /// accumulates toward a lockout. Only failures count against the limit.
    pub fn record_success(&self, key: &str) {
        if let Ok(mut windows) = self.windows.lock() {
            windows.remove(key);
        }
    }

    fn check_at(&self, key: &str, now: Instant) -> Result<(), DomainError> {
        let Ok(mut windows) = self.windows.lock() else {
            // A poisoned lock means another thread panicked while holding it.
            // Failing open would remove the protection precisely when something
            // is already wrong, so refuse instead.
            tracing::error!("rate limiter lock poisoned");
            return Err(DomainError::RateLimited);
        };

        if windows.len() > SWEEP_THRESHOLD {
            let window = self.window;
            windows.retain(|_, w| now.duration_since(w.started) < window);
        }

        let entry = windows.entry(key.to_owned()).or_insert(Window {
            started: now,
            count: 0,
        });

        if now.duration_since(entry.started) >= self.window {
            *entry = Window {
                started: now,
                count: 0,
            };
        }

        if entry.count >= self.max_attempts {
            return Err(DomainError::RateLimited);
        }

        entry.count += 1;
        Ok(())
    }
}

/// Build the key an attempt is counted against.
///
/// Both the address and the username appear, so exhausting the budget for one
/// account does not lock out a different account from the same address.
#[must_use]
pub fn attempt_key(ip: Option<std::net::IpAddr>, username: &str) -> String {
    let ip = ip.map_or_else(|| "unknown".to_owned(), |ip| ip.to_string());
    // Lowercased to match the case-insensitive uniqueness of the username
    // column; otherwise `Alice` and `alice` would get separate budgets for the
    // same account.
    format!("{ip}|{}", username.to_lowercase())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn limiter() -> RateLimiter {
        RateLimiter::new(3, Duration::from_mins(1))
    }

    #[test]
    fn attempts_within_the_budget_are_allowed() {
        let limiter = limiter();
        for _ in 0..3 {
            assert!(limiter.check("k").is_ok());
        }
    }

    #[test]
    fn the_attempt_after_the_budget_is_rejected() {
        let limiter = limiter();
        for _ in 0..3 {
            limiter.check("k").unwrap();
        }
        assert!(matches!(limiter.check("k"), Err(DomainError::RateLimited)));
    }

    #[test]
    fn the_budget_refills_once_the_window_passes() {
        let limiter = limiter();
        let start = Instant::now();

        for _ in 0..3 {
            limiter.check_at("k", start).unwrap();
        }
        assert!(limiter.check_at("k", start).is_err());

        let later = start + Duration::from_secs(61);
        assert!(limiter.check_at("k", later).is_ok());
    }

    #[test]
    fn the_budget_does_not_refill_early() {
        let limiter = limiter();
        let start = Instant::now();
        for _ in 0..3 {
            limiter.check_at("k", start).unwrap();
        }
        assert!(
            limiter
                .check_at("k", start + Duration::from_secs(59))
                .is_err()
        );
    }

    #[test]
    fn budgets_are_independent_per_key() {
        let limiter = limiter();
        for _ in 0..3 {
            limiter.check("a").unwrap();
        }
        assert!(limiter.check("a").is_err());
        // Exhausting one account must not lock out another.
        assert!(limiter.check("b").is_ok());
    }

    #[test]
    fn a_successful_sign_in_clears_the_count() {
        let limiter = limiter();
        limiter.check("k").unwrap();
        limiter.check("k").unwrap();
        limiter.record_success("k");

        // Full budget again, rather than two attempts from a lockout.
        for _ in 0..3 {
            assert!(limiter.check("k").is_ok());
        }
    }

    #[test]
    fn key_combines_address_and_username() {
        let ip = Some("203.0.113.7".parse().unwrap());
        assert_eq!(attempt_key(ip, "alice"), "203.0.113.7|alice");
        assert_ne!(attempt_key(ip, "alice"), attempt_key(ip, "bob"));
    }

    #[test]
    fn key_is_case_insensitive_in_the_username() {
        // `Alice` and `alice` are the same account, so they share one budget.
        let ip = Some("203.0.113.7".parse().unwrap());
        assert_eq!(attempt_key(ip, "Alice"), attempt_key(ip, "alice"));
    }

    #[test]
    fn an_unknown_address_still_produces_a_usable_key() {
        assert_eq!(attempt_key(None, "alice"), "unknown|alice");
        // Different accounts from unknown addresses remain independent, so one
        // unattributable attacker cannot lock out every account at once.
        assert_ne!(attempt_key(None, "alice"), attempt_key(None, "bob"));
    }

    #[test]
    fn stale_entries_are_swept_once_the_map_grows() {
        let limiter = RateLimiter::new(1, Duration::from_millis(1));
        let start = Instant::now();

        for i in 0..=SWEEP_THRESHOLD {
            let _ = limiter.check_at(&format!("key-{i}"), start);
        }

        // One window later a new attempt triggers the sweep, which must drop
        // the expired entries rather than let the map grow without bound.
        let later = start + Duration::from_secs(1);
        limiter.check_at("trigger", later).unwrap();

        let len = limiter.windows.lock().unwrap().len();
        assert!(
            len < SWEEP_THRESHOLD,
            "expected a sweep, still holding {len} entries"
        );
    }
}
