//! Environment loading and validation.
//!
//! This module is the single place where the process environment is read.
//! It loads the `.env` file (when present), reads every variable the service
//! needs, validates them, and returns a strongly-typed [`Config`]. Each failure
//! names the offending variable so startup problems are obvious from the logs.

use std::{env, error::Error, fmt, net::SocketAddr, num::ParseIntError, path::PathBuf};

use tracing::{debug, info, warn};

/// Minimum byte length required for the admin API key and password pepper.
const MIN_SECRET_LEN: usize = 32;
/// Allowed range (inclusive) for the access-token TTL, in seconds.
const TOKEN_TTL_RANGE: std::ops::RangeInclusive<u64> = 60..=3600;
/// Allowed range (inclusive) for the verified-JWT cache capacity.
const CACHE_CAPACITY_RANGE: std::ops::RangeInclusive<usize> = 1..=1_000_000;

const DEFAULT_TOKEN_TTL: u64 = 900;
const DEFAULT_CACHE_CAPACITY: usize = 10_000;
const DEFAULT_ADDR: &str = "127.0.0.1:8081";

/// Fully validated runtime configuration derived from the environment.
#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) signing_key_id: String,
    pub(crate) legacy_key_id: String,
    pub(crate) private_key_path: PathBuf,
    pub(crate) public_keys_dir: PathBuf,
    pub(crate) issuer: String,
    pub(crate) audience: String,
    pub(crate) admin_key: String,
    pub(crate) password_pepper: String,
    pub(crate) database_path: String,
    pub(crate) token_ttl: u64,
    pub(crate) cache_capacity: usize,
    pub(crate) addr: SocketAddr,
    pub(crate) allow_implicit: bool,
    pub(crate) cookie_secure: bool,
}

/// A configuration error that always names the variable (or rule) that failed.
#[derive(Debug)]
pub(crate) enum ConfigError {
    /// A required variable is absent or not valid UTF-8.
    Missing(&'static str),
    /// A variable is present but failed to parse into the expected type.
    Parse { var: &'static str, detail: String },
    /// A variable parsed but violated a validation rule.
    Invalid { var: &'static str, detail: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Missing(var) => {
                write!(f, "required environment variable `{var}` is not set")
            }
            ConfigError::Parse { var, detail } => {
                write!(f, "environment variable `{var}` is invalid: {detail}")
            }
            ConfigError::Invalid { var, detail } => {
                write!(f, "environment variable `{var}` is invalid: {detail}")
            }
        }
    }
}

impl Error for ConfigError {}

/// Read a required variable, mapping absence to a named [`ConfigError`].
fn required(var: &'static str) -> Result<String, ConfigError> {
    match env::var(var) {
        Ok(value) if !value.is_empty() => {
            debug!(var, "loaded required environment variable");
            Ok(value)
        }
        Ok(_) => Err(ConfigError::Missing(var)),
        Err(_) => Err(ConfigError::Missing(var)),
    }
}

/// Read an optional variable, returning `None` when unset.
fn optional(var: &'static str) -> Option<String> {
    match env::var(var) {
        Ok(value) if !value.is_empty() => {
            debug!(var, "loaded optional environment variable");
            Some(value)
        }
        _ => {
            debug!(var, "optional environment variable not set; using default");
            None
        }
    }
}

/// Parse an integer-valued variable, honoring a default when unset.
fn parse_with_default<T>(var: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr<Err = ParseIntError>,
{
    match optional(var) {
        None => Ok(default),
        Some(raw) => raw.parse::<T>().map_err(|error| ConfigError::Parse {
            var,
            detail: error.to_string(),
        }),
    }
}

impl Config {
    /// Load `.env` (best effort) and build a validated [`Config`].
    ///
    /// Logs each phase so that a failed startup pinpoints the culprit instead
    /// of exiting with the opaque "environment variable not found" message.
    pub(crate) fn load() -> Result<Self, ConfigError> {
        match dotenvy::dotenv() {
            Ok(path) => info!(path = %path.display(), "loaded .env file"),
            Err(error) if error.not_found() => {
                warn!("no .env file found; reading configuration from the process environment only")
            }
            Err(error) => warn!(%error, "failed to load .env file; continuing with process environment"),
        }

        info!("validating environment configuration");

        let signing_key_id = required("JWT_SIGNING_KEY_ID")?;
        let legacy_key_id =
            optional("JWT_LEGACY_KEY_ID").unwrap_or_else(|| signing_key_id.clone());
        let private_key_path = PathBuf::from(required("JWT_PRIVATE_KEY_PATH")?);
        let public_keys_dir = PathBuf::from(required("JWT_PUBLIC_KEYS_DIR")?);
        let issuer = required("JWT_ISSUER")?;
        let audience = required("JWT_AUDIENCE")?;
        let admin_key = required("AUTH_ADMIN_API_KEY")?;
        let password_pepper = required("AUTH_PASSWORD_PEPPER")?;
        let database_path = required("DATABASE_PATH")?;

        let token_ttl = parse_with_default("ACCESS_TOKEN_TTL_SECONDS", DEFAULT_TOKEN_TTL)?;
        let cache_capacity =
            parse_with_default("JWT_VERIFY_CACHE_CAPACITY", DEFAULT_CACHE_CAPACITY)?;

        let addr_raw = optional("AUTH_ADDR").unwrap_or_else(|| DEFAULT_ADDR.to_owned());
        let addr = addr_raw
            .parse::<SocketAddr>()
            .map_err(|error| ConfigError::Parse {
                var: "AUTH_ADDR",
                detail: error.to_string(),
            })?;

        let allow_implicit = env::var("ENABLE_IMPLICIT_FLOW")
            .is_ok_and(|value| value.eq_ignore_ascii_case("true"));
        let cookie_secure = !env::var("AUTH_COOKIE_SECURE")
            .is_ok_and(|value| value.eq_ignore_ascii_case("false"));

        // Validation rules with per-variable error messages.
        if admin_key.len() < MIN_SECRET_LEN {
            return Err(ConfigError::Invalid {
                var: "AUTH_ADMIN_API_KEY",
                detail: format!(
                    "must be at least {MIN_SECRET_LEN} bytes (got {})",
                    admin_key.len()
                ),
            });
        }
        if password_pepper.len() < MIN_SECRET_LEN {
            return Err(ConfigError::Invalid {
                var: "AUTH_PASSWORD_PEPPER",
                detail: format!(
                    "must be at least {MIN_SECRET_LEN} bytes (got {})",
                    password_pepper.len()
                ),
            });
        }
        if !TOKEN_TTL_RANGE.contains(&token_ttl) {
            return Err(ConfigError::Invalid {
                var: "ACCESS_TOKEN_TTL_SECONDS",
                detail: format!(
                    "must be between {} and {} seconds (got {token_ttl})",
                    TOKEN_TTL_RANGE.start(),
                    TOKEN_TTL_RANGE.end()
                ),
            });
        }
        if !CACHE_CAPACITY_RANGE.contains(&cache_capacity) {
            return Err(ConfigError::Invalid {
                var: "JWT_VERIFY_CACHE_CAPACITY",
                detail: format!(
                    "must be between {} and {} (got {cache_capacity})",
                    CACHE_CAPACITY_RANGE.start(),
                    CACHE_CAPACITY_RANGE.end()
                ),
            });
        }

        debug!(
            %addr,
            issuer = %issuer,
            audience = %audience,
            database_path = %database_path,
            public_keys_dir = %public_keys_dir.display(),
            signing_key_id = %signing_key_id,
            legacy_key_id = %legacy_key_id,
            token_ttl,
            cache_capacity,
            allow_implicit,
            cookie_secure,
            "environment configuration validated"
        );
        if !cookie_secure {
            warn!("AUTH_COOKIE_SECURE=false: cookies will be sent over plain HTTP (local dev only)");
        }
        if allow_implicit {
            warn!("ENABLE_IMPLICIT_FLOW=true: the legacy OAuth implicit flow is enabled");
        }

        Ok(Config {
            signing_key_id,
            legacy_key_id,
            private_key_path,
            public_keys_dir,
            issuer,
            audience,
            admin_key,
            password_pepper,
            database_path,
            token_ttl,
            cache_capacity,
            addr,
            allow_implicit,
            cookie_secure,
        })
    }
}
