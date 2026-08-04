use cookie::{Cookie, Expiration, SameSite};

use crate::auth::token::SessionToken;
use crate::config::Config;

/// Cookie name used when the connection is HTTPS.
///
/// The `__Host-` prefix is enforced by the browser, not by us: it refuses the
/// cookie unless it is `Secure`, has `Path=/`, and carries no `Domain`
/// attribute. That last point is what matters -- it makes the cookie
/// unsettable by a sibling subdomain, closing off session fixation from any
/// other host under the same registrable domain.
pub const SECURE_COOKIE_NAME: &str = "__Host-session";

/// Cookie name used in development.
///
/// Browsers reject `__Host-` cookies without `Secure`, and `Secure` cookies do
/// not survive plain `http://localhost`, so development needs an unprefixed
/// name. Production never uses this one.
pub const DEV_COOKIE_NAME: &str = "session";

/// The cookie name for the current environment.
#[must_use]
pub fn name(config: &Config) -> &'static str {
    if config.cookie_secure() {
        SECURE_COOKIE_NAME
    } else {
        DEV_COOKIE_NAME
    }
}

/// Build the `Set-Cookie` value that establishes a session.
///
/// The token lives here and nowhere else. It is never written to
/// `localStorage`, never serialized into the rendered HTML, and never returned
/// from a server function, so no amount of injected JavaScript can read it --
/// which is precisely what `HttpOnly` buys and what the previous
/// `localStorage` approach gave away.
#[must_use]
pub fn build(token: &SessionToken, config: &Config) -> String {
    let mut cookie = Cookie::new(name(config), token.expose().to_owned());
    apply_common_attributes(&mut cookie, config);

    // Max-Age rather than a bare session cookie, so the browser discards it on
    // the same schedule the server expires the row.
    cookie.set_max_age(Some(
        cookie::time::Duration::try_from(config.session_ttl)
            .unwrap_or(cookie::time::Duration::WEEK),
    ));

    cookie.to_string()
}

/// Build the `Set-Cookie` value that removes a session cookie.
///
/// Attributes must match the ones used when setting it, or the browser treats
/// this as a different cookie and leaves the original in place.
#[must_use]
pub fn clearing(config: &Config) -> String {
    let mut cookie = Cookie::new(name(config), "");
    apply_common_attributes(&mut cookie, config);
    cookie.set_expires(Expiration::DateTime(
        cookie::time::OffsetDateTime::UNIX_EPOCH,
    ));
    cookie.set_max_age(Some(cookie::time::Duration::ZERO));
    cookie.to_string()
}

/// Pull the session token out of a `Cookie` request header.
///
/// Returns `None` for a missing, malformed or wrongly-shaped value; callers
/// treat that as "not authenticated" rather than as an error, because an
/// absent cookie is the normal state for an anonymous visitor.
#[must_use]
pub fn extract(cookie_header: &str, config: &Config) -> Option<SessionToken> {
    let wanted = name(config);

    // `split_parse` rather than the percent-decoding variant: session tokens are
    // base64url, which needs no escaping, and skipping the decode step means a
    // crafted `%`-sequence cannot be decoded into something else on the way in.
    Cookie::split_parse(cookie_header)
        .filter_map(Result::ok)
        .find(|cookie| cookie.name() == wanted)
        .and_then(|cookie| SessionToken::from_cookie_value(cookie.value()))
}

fn apply_common_attributes(cookie: &mut Cookie<'_>, config: &Config) {
    cookie.set_http_only(true);
    cookie.set_secure(config.cookie_secure());
    cookie.set_path("/");

    // Lax, not Strict. Strict would withhold the cookie on any inbound
    // navigation from another site, so following a link to the app would
    // present as signed out. Lax still withholds it from cross-site POSTs,
    // which is the CSRF vector that matters for the mutating server functions.
    cookie.set_same_site(SameSite::Lax);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use core::time::Duration;
    use std::net::SocketAddr;

    use super::*;
    use crate::config::Environment;

    fn config(environment: Environment) -> Config {
        Config {
            database_url: "postgres://localhost/app".to_owned(),
            database_max_connections: 5,
            bind_addr: "127.0.0.1:3000".parse::<SocketAddr>().unwrap(),
            environment,
            session_ttl: Duration::from_hours(1),
            session_cleanup_interval: Duration::from_hours(1),
            trusted_proxy_hops: 0,
        }
    }

    #[test]
    fn production_cookie_carries_every_hardening_attribute() {
        let token = SessionToken::generate().unwrap();
        let header = build(&token, &config(Environment::Production));

        assert!(header.starts_with("__Host-session="), "got {header}");
        assert!(header.contains("HttpOnly"), "got {header}");
        assert!(header.contains("Secure"), "got {header}");
        assert!(header.contains("SameSite=Lax"), "got {header}");
        assert!(header.contains("Path=/"), "got {header}");
        assert!(header.contains("Max-Age=3600"), "got {header}");
        // A Domain attribute would invalidate the __Host- prefix.
        assert!(!header.contains("Domain"), "got {header}");
    }

    #[test]
    fn development_cookie_drops_secure_so_it_survives_plain_http() {
        let token = SessionToken::generate().unwrap();
        let header = build(&token, &config(Environment::Development));

        assert!(header.starts_with("session="), "got {header}");
        assert!(header.contains("HttpOnly"), "got {header}");
        assert!(!header.contains("Secure"), "got {header}");
        assert!(!header.contains("__Host-"), "got {header}");
    }

    #[test]
    fn cookie_is_always_http_only_in_both_environments() {
        // The single attribute that must never be relaxed for convenience.
        let token = SessionToken::generate().unwrap();
        for environment in [Environment::Development, Environment::Production] {
            assert!(build(&token, &config(environment)).contains("HttpOnly"));
        }
    }

    #[test]
    fn clearing_cookie_matches_the_attributes_used_when_setting() {
        let cfg = config(Environment::Production);
        let clear = clearing(&cfg);

        assert!(clear.starts_with("__Host-session="), "got {clear}");
        assert!(clear.contains("Max-Age=0"), "got {clear}");
        assert!(clear.contains("HttpOnly"), "got {clear}");
        assert!(clear.contains("Secure"), "got {clear}");
        assert!(clear.contains("Path=/"), "got {clear}");
    }

    #[test]
    fn extract_round_trips_a_cookie_we_issued() {
        let cfg = config(Environment::Production);
        let token = SessionToken::generate().unwrap();
        let request_header = format!("__Host-session={}", token.expose());

        assert_eq!(extract(&request_header, &cfg).unwrap(), token);
    }

    #[test]
    fn extract_finds_the_cookie_among_others() {
        let cfg = config(Environment::Production);
        let token = SessionToken::generate().unwrap();
        let header = format!("theme=dark; __Host-session={}; locale=en", token.expose());

        assert_eq!(extract(&header, &cfg).unwrap(), token);
    }

    #[test]
    fn extract_ignores_cookies_belonging_to_the_other_environment() {
        // A development cookie must not authenticate against a production
        // config, or vice versa.
        let token = SessionToken::generate().unwrap();
        let dev_header = format!("session={}", token.expose());

        assert!(extract(&dev_header, &config(Environment::Production)).is_none());
        assert!(extract(&dev_header, &config(Environment::Development)).is_some());
    }

    #[test]
    fn extract_rejects_malformed_values() {
        let cfg = config(Environment::Production);

        assert!(extract("", &cfg).is_none());
        assert!(extract("__Host-session=", &cfg).is_none());
        assert!(extract("__Host-session=nonsense", &cfg).is_none());
        assert!(extract("unrelated=value", &cfg).is_none());
        assert!(extract("__Host-session=' OR 1=1 --", &cfg).is_none());
    }

    #[test]
    fn issued_cookie_can_be_read_back_by_the_extractor() {
        // Guards against the set and parse paths drifting apart, which would
        // sign every user out on their next request.
        for environment in [Environment::Development, Environment::Production] {
            let cfg = config(environment);
            let token = SessionToken::generate().unwrap();
            let set_cookie = build(&token, &cfg);

            let request_header = set_cookie
                .split(';')
                .next()
                .expect("cookie header has a name=value pair");

            assert_eq!(extract(request_header, &cfg).unwrap(), token);
        }
    }
}
