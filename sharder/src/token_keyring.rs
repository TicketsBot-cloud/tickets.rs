//! Loads the whitelabel token keyring from the environment. It is kept out of
//! `Config` because that type derives `Debug` and is logged and traced.

use common::token_crypto::{KeyConfigError, TokenKeyring};
use std::fmt;

pub const KEYS_ENV: &str = "WHITELABEL_TOKEN_KEYS";
pub const ACTIVE_VERSION_ENV: &str = "WHITELABEL_TOKEN_ACTIVE_KEY_VERSION";

/// Never carries any part of the configured values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringLoadError {
    KeysMissing,
    ActiveVersionMissing,
    Invalid(KeyConfigError),
}

impl fmt::Display for KeyringLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeysMissing => write!(f, "{} is not set", KEYS_ENV),
            Self::ActiveVersionMissing => write!(f, "{} is not set", ACTIVE_VERSION_ENV),
            Self::Invalid(e) => write!(f, "{} / {}: {}", KEYS_ENV, ACTIVE_VERSION_ENV, e),
        }
    }
}

impl std::error::Error for KeyringLoadError {}

pub fn load_keyring(
    keys: Option<&str>,
    active_version: Option<&str>,
) -> Result<TokenKeyring, KeyringLoadError> {
    let keys = keys
        .filter(|v| !v.trim().is_empty())
        .ok_or(KeyringLoadError::KeysMissing)?;
    let active_version = active_version
        .filter(|v| !v.trim().is_empty())
        .ok_or(KeyringLoadError::ActiveVersionMissing)?;

    TokenKeyring::parse(keys, active_version).map_err(KeyringLoadError::Invalid)
}

pub fn load_keyring_from_env() -> Result<TokenKeyring, KeyringLoadError> {
    let keys = std::env::var(KEYS_ENV).ok();
    let active_version = std::env::var(ACTIVE_VERSION_ENV).ok();

    load_keyring(keys.as_deref(), active_version.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_B64: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    #[test]
    fn loads_valid_configuration() {
        let keys = format!("1:{}", KEY_B64);
        let ring = load_keyring(Some(&keys), Some("1")).unwrap();
        assert_eq!(ring.active_version(), 1);
    }

    #[test]
    fn rejects_missing_or_blank_values() {
        let keys = format!("1:{}", KEY_B64);
        assert_eq!(
            load_keyring(None, Some("1")).unwrap_err(),
            KeyringLoadError::KeysMissing
        );
        assert_eq!(
            load_keyring(Some("  "), Some("1")).unwrap_err(),
            KeyringLoadError::KeysMissing
        );
        assert_eq!(
            load_keyring(Some(&keys), None).unwrap_err(),
            KeyringLoadError::ActiveVersionMissing
        );
        assert_eq!(
            load_keyring(Some(&keys), Some("")).unwrap_err(),
            KeyringLoadError::ActiveVersionMissing
        );
    }

    #[test]
    fn rejects_malformed_values_without_echoing_them() {
        let secret = "supersecretkeymaterial";
        let bad = [
            (format!("1:{}", secret), "1"),
            (format!("1:{}", KEY_B64), "2"),
            (format!("1:{}", KEY_B64), "abc"),
            (secret.to_owned(), "1"),
        ];

        for (keys, active) in bad {
            let err = load_keyring(Some(&keys), Some(active)).unwrap_err();
            let msg = format!("{} {:?}", err, err);
            assert!(!msg.contains(secret), "{}", msg);
            assert!(!msg.contains(KEY_B64), "{}", msg);
        }
    }
}
