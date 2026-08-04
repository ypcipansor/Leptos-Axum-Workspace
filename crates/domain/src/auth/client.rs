use std::net::{IpAddr, SocketAddr};

use http::{HeaderMap, HeaderName};

use crate::config::Config;
use crate::repo::sessions::ClientContext;

const FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

/// Longest `User-Agent` we will store.
///
/// The header is attacker-controlled and unbounded; the column is not meant to
/// hold a kilobyte of it.
const MAX_USER_AGENT_LEN: usize = 400;

/// Work out who is making this request, for the session audit trail.
///
/// # Trusting `X-Forwarded-For`
///
/// The header is trivially forgeable by whoever sends the request, so it is
/// only consulted when the deployment declares how many reverse proxies sit in
/// front of the process, via `TRUSTED_PROXY_HOPS`.
///
/// - `0` (the default): the header is ignored entirely and the TCP peer address
///   is used. Correct when the process is directly exposed.
/// - `n`: skip the `n` entries the trusted proxies appended and take the one
///   before them, which is the address the outermost trusted proxy actually
///   observed.
///
/// The previous implementation took the *leftmost* entry unconditionally --
/// the one field of the header entirely under the caller's control -- and then
/// substituted `127.0.0.1` when it was absent, so every stored address was
/// either forgeable or fabricated. Here an address that cannot be established
/// is recorded as `None`.
#[must_use]
pub fn resolve_context(
    headers: &HeaderMap,
    peer: Option<SocketAddr>,
    config: &Config,
) -> ClientContext {
    ClientContext {
        user_agent: extract_user_agent(headers),
        ip_address: resolve_ip(headers, peer, config.trusted_proxy_hops),
    }
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            // Truncate on a character boundary; slicing bytes could split a
            // multi-byte sequence and panic.
            value.chars().take(MAX_USER_AGENT_LEN).collect()
        })
}

fn resolve_ip(
    headers: &HeaderMap,
    peer: Option<SocketAddr>,
    trusted_hops: usize,
) -> Option<IpAddr> {
    let peer_ip = peer.map(|addr| addr.ip());

    if trusted_hops == 0 {
        return peer_ip;
    }

    let forwarded: Vec<&str> = headers
        .get_all(FORWARDED_FOR)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect();

    // Fewer entries than trusted hops means the chain is shorter than declared,
    // so nothing in the header can be attributed to a trusted proxy. Fall back
    // to the peer rather than trusting a value we cannot place.
    let index = match forwarded.len().checked_sub(trusted_hops) {
        Some(index) if index < forwarded.len() => index,
        _ => return peer_ip,
    };

    forwarded
        .get(index)
        .and_then(|entry| parse_forwarded_entry(entry))
        .or(peer_ip)
}

/// Parse one `X-Forwarded-For` entry.
///
/// Handles the bare-address form as well as the bracketed IPv6-with-port form
/// some proxies emit (`[2001:db8::1]:443`).
fn parse_forwarded_entry(entry: &str) -> Option<IpAddr> {
    if let Ok(ip) = entry.parse::<IpAddr>() {
        return Some(ip);
    }
    if let Ok(addr) = entry.parse::<SocketAddr>() {
        return Some(addr.ip());
    }
    // `192.0.2.1:443`
    if let Some((host, _port)) = entry.rsplit_once(':')
        && let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>()
    {
        return Some(ip);
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::config::Environment;

    fn config(trusted_proxy_hops: usize) -> Config {
        Config {
            database_url: "postgres://localhost/app".to_owned(),
            database_max_connections: 5,
            bind_addr: "127.0.0.1:3000".parse().unwrap(),
            environment: Environment::Production,
            session_ttl: Duration::from_hours(1),
            session_cleanup_interval: Duration::from_hours(1),
            trusted_proxy_hops,
        }
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        map
    }

    fn peer(addr: &str) -> Option<SocketAddr> {
        addr.parse().ok()
    }

    #[test]
    fn without_trusted_proxies_the_forwarded_header_is_ignored() {
        // The core fix: a caller cannot choose what gets recorded about them.
        let h = headers(&[("x-forwarded-for", "1.2.3.4")]);
        let resolved = resolve_ip(&h, peer("203.0.113.9:5000"), 0);
        assert_eq!(resolved, Some("203.0.113.9".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn spoofed_header_cannot_override_the_peer_address() {
        let h = headers(&[("x-forwarded-for", "127.0.0.1, 10.0.0.1, 8.8.8.8")]);
        assert_eq!(
            resolve_ip(&h, peer("203.0.113.9:5000"), 0),
            Some("203.0.113.9".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn one_trusted_proxy_takes_the_address_it_appended() {
        // nginx with $proxy_add_x_forwarded_for and a directly connected client.
        let h = headers(&[("x-forwarded-for", "198.51.100.7")]);
        assert_eq!(
            resolve_ip(&h, peer("10.0.0.2:5000"), 1),
            Some("198.51.100.7".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn two_trusted_proxies_skip_both_appended_entries() {
        // CDN -> nginx -> app. The CDN recorded the client; nginx appended the CDN.
        let h = headers(&[("x-forwarded-for", "198.51.100.7, 203.0.113.50")]);
        assert_eq!(
            resolve_ip(&h, peer("10.0.0.2:5000"), 2),
            Some("198.51.100.7".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn extra_client_supplied_entries_are_not_trusted() {
        // The client prepended two fabricated hops. With one trusted proxy we
        // still read only the entry our proxy added.
        let h = headers(&[("x-forwarded-for", "1.1.1.1, 2.2.2.2, 198.51.100.7")]);
        assert_eq!(
            resolve_ip(&h, peer("10.0.0.2:5000"), 1),
            Some("198.51.100.7".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn a_chain_shorter_than_declared_falls_back_to_the_peer() {
        // Declaring two hops but receiving one entry means the request did not
        // arrive the way the deployment says it does. Trusting it anyway would
        // let a direct connection choose its own recorded address.
        let h = headers(&[("x-forwarded-for", "1.1.1.1")]);
        assert_eq!(
            resolve_ip(&h, peer("10.0.0.2:5000"), 2),
            Some("10.0.0.2".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn missing_header_with_trusted_proxies_falls_back_to_the_peer() {
        assert_eq!(
            resolve_ip(&HeaderMap::new(), peer("10.0.0.2:5000"), 1),
            Some("10.0.0.2".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn unknown_address_is_recorded_as_none_not_as_localhost() {
        // The previous implementation substituted 127.0.0.1 here, which made
        // the audit trail actively misleading.
        assert_eq!(resolve_ip(&HeaderMap::new(), None, 0), None);
        assert_eq!(resolve_ip(&HeaderMap::new(), None, 1), None);
    }

    #[test]
    fn garbage_entries_do_not_produce_an_address() {
        let h = headers(&[("x-forwarded-for", "not-an-ip")]);
        assert_eq!(resolve_ip(&h, None, 1), None);
    }

    #[test]
    fn entries_split_across_repeated_headers_are_joined() {
        let h = headers(&[
            ("x-forwarded-for", "198.51.100.7"),
            ("x-forwarded-for", "203.0.113.50"),
        ]);
        assert_eq!(
            resolve_ip(&h, peer("10.0.0.2:5000"), 2),
            Some("198.51.100.7".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn ipv6_forms_are_understood() {
        let h = headers(&[("x-forwarded-for", "2001:db8::1")]);
        assert_eq!(
            resolve_ip(&h, None, 1),
            Some("2001:db8::1".parse::<IpAddr>().unwrap())
        );

        let bracketed = headers(&[("x-forwarded-for", "[2001:db8::1]:443")]);
        assert_eq!(
            resolve_ip(&bracketed, None, 1),
            Some("2001:db8::1".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn ipv4_with_port_is_understood() {
        let h = headers(&[("x-forwarded-for", "198.51.100.7:44321")]);
        assert_eq!(
            resolve_ip(&h, None, 1),
            Some("198.51.100.7".parse::<IpAddr>().unwrap())
        );
    }

    #[test]
    fn user_agent_is_captured_and_bounded() {
        let h = headers(&[("user-agent", "Mozilla/5.0 (X11; Linux x86_64)")]);
        let context = resolve_context(&h, None, &config(0));
        assert_eq!(
            context.user_agent.as_deref(),
            Some("Mozilla/5.0 (X11; Linux x86_64)")
        );

        let long = "U".repeat(MAX_USER_AGENT_LEN * 2);
        let h = headers(&[("user-agent", long.as_str())]);
        let context = resolve_context(&h, None, &config(0));
        assert_eq!(
            context.user_agent.map(|ua| ua.chars().count()),
            Some(MAX_USER_AGENT_LEN)
        );
    }

    #[test]
    fn non_ascii_user_agent_is_discarded_rather_than_stored() {
        // `HeaderValue::to_str` only succeeds for visible ASCII, which is what
        // RFC 9110 permits in a field value. A header carrying raw UTF-8 is
        // dropped instead of being lossily decoded -- real clients never send
        // one, and guessing at an encoding would put attacker-chosen bytes into
        // a column the session list renders.
        let long: String = "日".repeat(MAX_USER_AGENT_LEN * 2);
        let h = headers(&[("user-agent", long.as_str())]);
        assert_eq!(resolve_context(&h, None, &config(0)).user_agent, None);
    }

    #[test]
    fn truncation_happens_on_a_character_boundary() {
        // Guards the `chars().take(..)` in extract_user_agent. Byte slicing
        // would panic here if a non-ASCII value ever reached it.
        let value = format!("{}{}", "a".repeat(MAX_USER_AGENT_LEN - 1), "x".repeat(50));
        let h = headers(&[("user-agent", value.as_str())]);
        let stored = resolve_context(&h, None, &config(0)).user_agent.unwrap();
        assert_eq!(stored.chars().count(), MAX_USER_AGENT_LEN);
        assert!(stored.is_char_boundary(stored.len()));
    }

    #[test]
    fn absent_or_blank_user_agent_is_none() {
        assert_eq!(
            resolve_context(&HeaderMap::new(), None, &config(0)).user_agent,
            None
        );
        let h = headers(&[("user-agent", "   ")]);
        assert_eq!(resolve_context(&h, None, &config(0)).user_agent, None);
    }
}
