use core::fmt;
use core::str::FromStr;
use core::time::Duration;
use std::net::SocketAddr;

/// Which environment the process believes it is running in.
///
/// This drives defaults that must not be guessed wrong: cookies are only
/// allowed over plain HTTP in development, and logs are only human-formatted
/// there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    #[must_use]
    pub const fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

impl FromStr for Environment {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dev" | "development" | "local" => Ok(Self::Development),
            "prod" | "production" => Ok(Self::Production),
            other => Err(ConfigError::Invalid {
                key: "APP_ENV",
                reason: format!("expected one of development|production, got `{other}`"),
            }),
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Development => "development",
            Self::Production => "production",
        })
    }
}

/// Everything the process needs from its environment, resolved once at startup.
///
/// Reading configuration in one place, at boot, means a misconfigured
/// deployment fails immediately and loudly rather than at the first request
/// that happens to touch the missing value. The previous code called
/// `env::var` from several modules and fell back to a hard-coded database URL
/// containing credentials.
// `Debug` is implemented by hand below rather than derived, so the connection
// string cannot reach a log line.
#[derive(Clone)]
pub struct Config {
    /// Postgres connection string. Never logged: `Debug` is implemented by
    /// hand below so the password cannot reach a log line.
    pub database_url: String,
    pub database_max_connections: u32,
    pub bind_addr: SocketAddr,
    pub environment: Environment,
    /// How long a newly issued session stays valid.
    pub session_ttl: Duration,
    /// How often the background sweep deletes expired sessions.
    pub session_cleanup_interval: Duration,
    /// Number of trusted reverse proxies in front of this process.
    ///
    /// `0` -- the default -- means `X-Forwarded-For` is ignored entirely and
    /// the peer address is used. Anything else takes the Nth address from the
    /// right of the header. The previous code took the leftmost value, which is
    /// fully attacker-controlled, then substituted `127.0.0.1` when absent.
    pub trusted_proxy_hops: usize,
}

impl Config {
    /// Read and validate configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = optional("APP_ENV")
            .map(|raw| raw.parse::<Environment>())
            .transpose()?
            .unwrap_or(Environment::Development);

        let database_url = required("DATABASE_URL")?;
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(ConfigError::Invalid {
                key: "DATABASE_URL",
                reason: "must be a postgres:// or postgresql:// URL".to_owned(),
            });
        }

        let host = optional("HOST").unwrap_or_else(|| "0.0.0.0".to_owned());
        let port = parse_or("PORT", 3000_u16)?;
        let bind_addr = format!("{host}:{port}")
            .parse::<SocketAddr>()
            .map_err(|e| ConfigError::Invalid {
                key: "HOST/PORT",
                reason: e.to_string(),
            })?;

        let session_ttl = Duration::from_secs(parse_or("SESSION_TTL_SECONDS", 60 * 60 * 24 * 7)?);
        if session_ttl.is_zero() {
            return Err(ConfigError::Invalid {
                key: "SESSION_TTL_SECONDS",
                reason: "must be greater than zero".to_owned(),
            });
        }

        let session_cleanup_interval =
            Duration::from_secs(parse_or("SESSION_CLEANUP_INTERVAL_SECONDS", 60 * 60)?);
        if session_cleanup_interval.is_zero() {
            return Err(ConfigError::Invalid {
                key: "SESSION_CLEANUP_INTERVAL_SECONDS",
                reason: "must be greater than zero".to_owned(),
            });
        }

        Ok(Self {
            database_url,
            database_max_connections: parse_or("DATABASE_MAX_CONNECTIONS", 10_u32)?,
            bind_addr,
            environment,
            session_ttl,
            session_cleanup_interval,
            trusted_proxy_hops: parse_or("TRUSTED_PROXY_HOPS", 0_usize)?,
        })
    }

    /// Whether the session cookie should carry the `Secure` attribute.
    ///
    /// Always true in production. In development it is relaxed so the app works
    /// over `http://localhost`, where browsers would otherwise drop the cookie.
    #[must_use]
    pub const fn cookie_secure(&self) -> bool {
        self.environment.is_production()
    }
}

// Hand-written so the database password never reaches a log line, a panic
// message, or a `#[derive(Debug)]` on any struct that embeds Config.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("database_url", &"<redacted>")
            .field("database_max_connections", &self.database_max_connections)
            .field("bind_addr", &self.bind_addr)
            .field("environment", &self.environment)
            .field("session_ttl", &self.session_ttl)
            .field("session_cleanup_interval", &self.session_cleanup_interval)
            .field("trusted_proxy_hops", &self.trusted_proxy_hops)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("required environment variable `{0}` is not set")]
    Missing(&'static str),

    #[error("environment variable `{key}` is invalid: {reason}")]
    Invalid { key: &'static str, reason: String },
}

fn optional(key: &'static str) -> Option<String> {
    // An empty variable is treated as unset. Container orchestrators routinely
    // inject `KEY=` for absent values, and the previous code crashed on it.
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn required(key: &'static str) -> Result<String, ConfigError> {
    optional(key).ok_or(ConfigError::Missing(key))
}

fn parse_or<T>(key: &'static str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    match optional(key) {
        None => Ok(default),
        Some(raw) => raw.parse::<T>().map_err(|e| ConfigError::Invalid {
            key,
            reason: e.to_string(),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn environment_parses_accepted_spellings() {
        assert_eq!(
            "dev".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!(
            "Development".parse::<Environment>().unwrap(),
            Environment::Development
        );
        assert_eq!(
            "prod".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert_eq!(
            " PRODUCTION ".parse::<Environment>().unwrap(),
            Environment::Production
        );
    }

    #[test]
    fn environment_rejects_unknown_values() {
        // Silently defaulting a typo like APP_ENV=produciton to development
        // would ship insecure cookie settings to production.
        assert!("staging".parse::<Environment>().is_err());
        assert!("".parse::<Environment>().is_err());
    }

    #[test]
    fn cookie_secure_follows_environment() {
        assert!(Environment::Production.is_production());
        assert!(!Environment::Development.is_production());
    }

    #[test]
    fn debug_output_hides_the_database_password() {
        let config = Config {
            database_url: "postgres://user:hunter2@db:5432/app".to_owned(),
            database_max_connections: 10,
            bind_addr: "127.0.0.1:3000".parse().unwrap(),
            environment: Environment::Production,
            session_ttl: Duration::from_mins(1),
            session_cleanup_interval: Duration::from_mins(1),
            trusted_proxy_hops: 0,
        };

        let rendered = format!("{config:?}");
        assert!(!rendered.contains("hunter2"), "password leaked: {rendered}");
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn parse_or_returns_default_when_unset() {
        // Name chosen to be absent from any real environment.
        assert_eq!(parse_or("APP_TEST_DEFINITELY_UNSET_KEY", 7_u16).unwrap(), 7);
    }
}
