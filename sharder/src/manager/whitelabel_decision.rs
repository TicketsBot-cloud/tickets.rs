//! Pure decisions for the whitelabel connect loop, separated so the safety
//! rules around credentials can be tested without a database or gateway.

use common::token_crypto::{StoredToken, TokenCryptoError, TokenKeyring};
use database::WhitelabelBot;

#[derive(Debug, PartialEq, Eq)]
pub enum RowAction {
    Connect(String),
    /// The stored credentials could not be turned into a token. This is a
    /// local fault (missing key, corrupt row) and says nothing about whether
    /// Discord would accept the bot, so it must never delete the row, record
    /// an authentication error or fall back to another token.
    BackOff(TokenCryptoError),
}

pub fn row_action(keyring: &TokenKeyring, bot: &WhitelabelBot) -> RowAction {
    let stored = StoredToken {
        bot_id: bot.bot_id as u64,
        legacy_token: bot.token.as_deref(),
        ciphertext: bot.token_ciphertext.as_deref(),
        key_version: bot.token_key_version,
    };

    match keyring.resolve_stored(&stored) {
        Ok(token) => RowAction::Connect(token),
        Err(e) => RowAction::BackOff(e),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthFailureAction {
    /// The row still held the credentials Discord rejected and was deleted.
    RecordAndStop,
    /// The row changed while connected (for example a rotation), so the
    /// failure refers to stale credentials. Reconnect with the new row.
    Reconnect,
    /// The conditional delete itself failed. Leave the row alone and retry.
    RetryLater,
}

pub fn auth_failure_action<E>(delete_result: &Result<bool, E>) -> AuthFailureAction {
    match delete_result {
        Ok(true) => AuthFailureAction::RecordAndStop,
        Ok(false) => AuthFailureAction::Reconnect,
        Err(_) => AuthFailureAction::RetryLater,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_B64: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    fn keyring() -> TokenKeyring {
        TokenKeyring::parse(&format!("1:{}", KEY_B64), "1").unwrap()
    }

    fn row(
        token: Option<&str>,
        ciphertext: Option<Vec<u8>>,
        version: Option<i16>,
    ) -> WhitelabelBot {
        WhitelabelBot {
            user_id: 10,
            bot_id: 20,
            public_key: "pk".to_owned(),
            token: token.map(str::to_owned),
            token_ciphertext: ciphertext,
            token_key_version: version,
        }
    }

    #[test]
    fn decrypts_encrypted_row() {
        let ring = keyring();
        let (sealed, version) = ring.seal(20, "the.bot.token").unwrap();

        assert_eq!(
            row_action(&ring, &row(None, Some(sealed), Some(version))),
            RowAction::Connect("the.bot.token".to_owned())
        );
    }

    #[test]
    fn legacy_token_wins_over_ciphertext() {
        let ring = keyring();
        let (sealed, version) = ring.seal(20, "old.token").unwrap();

        assert_eq!(
            row_action(&ring, &row(Some("new.token"), Some(sealed), Some(version))),
            RowAction::Connect("new.token".to_owned())
        );
    }

    #[test]
    fn decrypt_failures_back_off_and_never_connect() {
        let ring = keyring();
        let (sealed, version) = ring.seal(20, "the.bot.token").unwrap();

        let mut tampered = sealed.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xff;

        let cases = [
            (
                "tampered ciphertext",
                row(None, Some(tampered), Some(version)),
                TokenCryptoError::DecryptFailed,
            ),
            (
                "unknown key version",
                row(None, Some(sealed.clone()), Some(9)),
                TokenCryptoError::UnknownKeyVersion,
            ),
            (
                "truncated ciphertext",
                row(None, Some(vec![1, 2, 3]), Some(version)),
                TokenCryptoError::SealedTooShort,
            ),
            (
                "no token at all",
                row(None, None, None),
                TokenCryptoError::MissingToken,
            ),
            (
                "empty legacy and no ciphertext",
                row(Some(""), None, None),
                TokenCryptoError::MissingToken,
            ),
            (
                "ciphertext without version",
                row(None, Some(sealed), None),
                TokenCryptoError::MissingToken,
            ),
        ];

        for (name, bot, want) in cases {
            assert_eq!(
                row_action(&ring, &bot),
                RowAction::BackOff(want),
                "{}",
                name
            );
        }
    }

    #[test]
    fn ciphertext_is_bound_to_its_bot() {
        let ring = keyring();
        let (sealed, version) = ring.seal(999, "the.bot.token").unwrap();

        assert_eq!(
            row_action(&ring, &row(None, Some(sealed), Some(version))),
            RowAction::BackOff(TokenCryptoError::DecryptFailed)
        );
    }

    #[test]
    fn conditional_delete_decisions() {
        assert_eq!(
            auth_failure_action::<()>(&Ok(true)),
            AuthFailureAction::RecordAndStop
        );
        assert_eq!(
            auth_failure_action::<()>(&Ok(false)),
            AuthFailureAction::Reconnect
        );
        assert_eq!(
            auth_failure_action(&Err("db down")),
            AuthFailureAction::RetryLater
        );
    }
}
