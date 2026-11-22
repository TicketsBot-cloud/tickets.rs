//! Versioned AES-256-GCM sealing for whitelabel bot tokens.
//!
//! The sealed layout is `nonce (12) || ciphertext || tag (16)`, with the bot id
//! (8 bytes, big endian) as additional authenticated data so a sealed value
//! cannot be moved between rows. This must stay byte compatible with the Go
//! implementation in `common/encryption/versioned.go`; the shared vectors in
//! `testdata/token_vectors.json` enforce that.

use std::collections::HashMap;
use std::fmt;

use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};

pub const MIN_SEALED_LEN: usize = NONCE_LEN + TAG_LEN;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const MIN_KEY_VERSION: u32 = 1;
const MAX_KEY_VERSION: u32 = 32767;

/// Configuration errors. Variants carry only the 1-based entry position, never
/// any part of the configured value, so key material cannot reach logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyConfigError {
    Empty,
    InvalidActiveVersion,
    MalformedEntry(usize),
    InvalidVersion(usize),
    InvalidBase64(usize),
    WrongKeyLength(usize),
    DuplicateVersion(usize),
    ActiveVersionMissing,
}

impl fmt::Display for KeyConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "invalid key configuration: no keys provided"),
            Self::InvalidActiveVersion => {
                write!(f, "invalid key configuration: active version is not a valid version number")
            }
            Self::MalformedEntry(n) => write!(f, "invalid key configuration: entry {} is malformed", n),
            Self::InvalidVersion(n) => {
                write!(f, "invalid key configuration: entry {} has an invalid version", n)
            }
            Self::InvalidBase64(n) => {
                write!(f, "invalid key configuration: entry {} is not canonical base64", n)
            }
            Self::WrongKeyLength(n) => {
                write!(f, "invalid key configuration: entry {} is not {} bytes", n, KEY_LEN)
            }
            Self::DuplicateVersion(n) => {
                write!(f, "invalid key configuration: entry {} repeats an earlier version", n)
            }
            Self::ActiveVersionMissing => write!(f, "invalid key configuration: active version has no key"),
        }
    }
}

impl std::error::Error for KeyConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCryptoError {
    SealedTooShort,
    DecryptFailed,
    UnknownKeyVersion,
    MissingToken,
    EmptyToken,
    Random,
}

impl fmt::Display for TokenCryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::SealedTooShort => "decryption failed: sealed value too short",
            Self::DecryptFailed => "decryption failed",
            Self::UnknownKeyVersion => "unknown key version",
            Self::MissingToken => "no usable token stored",
            Self::EmptyToken => "refusing to seal an empty token",
            Self::Random => "failed to generate a nonce",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for TokenCryptoError {}

/// A stored token row as read from the database. The lifetimes borrow from the
/// row so resolving does not copy ciphertext.
#[derive(Clone, Copy)]
pub struct StoredToken<'a> {
    pub bot_id: u64,
    pub legacy_token: Option<&'a str>,
    pub ciphertext: Option<&'a [u8]>,
    pub key_version: Option<i16>,
}

impl fmt::Debug for StoredToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredToken")
            .field("bot_id", &self.bot_id)
            .field("legacy_token", &self.legacy_token.map(|_| "<redacted>"))
            .field("ciphertext_len", &self.ciphertext.map(|c| c.len()))
            .field("key_version", &self.key_version)
            .finish()
    }
}

pub struct TokenKeyring {
    keys: HashMap<i16, LessSafeKey>,
    active: i16,
}

impl fmt::Debug for TokenKeyring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TokenKeyring{redacted}")
    }
}

pub fn bot_id_aad(bot_id: u64) -> [u8; 8] {
    bot_id.to_be_bytes()
}

fn parse_version(s: &str) -> Option<i16> {
    let b = s.as_bytes();
    if b.is_empty() || b.len() > 5 || b[0] == b'0' || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }

    let n = b.iter().fold(0u32, |acc, c| acc * 10 + u32::from(c - b'0'));
    if !(MIN_KEY_VERSION..=MAX_KEY_VERSION).contains(&n) {
        return None;
    }

    Some(n as i16)
}

/// Strict standard base64: padded, canonical (zero trailing bits), and no
/// characters outside the alphabet. Mirrors the Go decode-then-reencode check.
fn decode_base64_strict(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    if b.is_empty() || b.len() % 4 != 0 {
        return None;
    }

    let pad = b.iter().rev().take_while(|c| **c == b'=').count();
    if pad > 2 {
        return None;
    }

    let data = &b[..b.len() - pad];
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;

    for c in data {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }

    if acc != 0 {
        return None;
    }

    Some(out)
}

fn make_key(key: &[u8]) -> Option<LessSafeKey> {
    UnboundKey::new(&AES_256_GCM, key).ok().map(LessSafeKey::new)
}

/// Split from `seal` so tests can reproduce fixed vectors.
fn seal_with_nonce(
    key: &LessSafeKey,
    nonce: [u8; NONCE_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, TokenCryptoError> {
    let mut body = plaintext.to_vec();
    key.seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::from(aad), &mut body)
        .map_err(|_| TokenCryptoError::DecryptFailed)?;

    let mut out = Vec::with_capacity(NONCE_LEN + body.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&body);
    Ok(out)
}

fn open_sealed(key: &LessSafeKey, sealed: &[u8], aad: &[u8]) -> Result<Vec<u8>, TokenCryptoError> {
    if sealed.len() < MIN_SEALED_LEN {
        return Err(TokenCryptoError::SealedTooShort);
    }

    let (nonce_bytes, rest) = sealed.split_at(NONCE_LEN);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(nonce_bytes);

    let mut buf = rest.to_vec();
    let plaintext = key
        .open_in_place(Nonce::assume_unique_for_key(nonce), Aad::from(aad), &mut buf)
        .map_err(|_| TokenCryptoError::DecryptFailed)?;

    Ok(plaintext.to_vec())
}

impl TokenKeyring {
    /// Parses `1:<base64>,2:<base64>` and selects the active version.
    pub fn parse(keys: &str, active: &str) -> Result<Self, KeyConfigError> {
        if keys.trim().is_empty() {
            return Err(KeyConfigError::Empty);
        }

        let active = parse_version(active.trim()).ok_or(KeyConfigError::InvalidActiveVersion)?;
        let mut map = HashMap::new();

        for (i, entry) in keys.split(',').enumerate() {
            let n = i + 1;
            let (version_str, key_str) = entry
                .trim()
                .split_once(':')
                .ok_or(KeyConfigError::MalformedEntry(n))?;

            let version = parse_version(version_str).ok_or(KeyConfigError::InvalidVersion(n))?;
            let key = decode_base64_strict(key_str).ok_or(KeyConfigError::InvalidBase64(n))?;

            if key.len() != KEY_LEN {
                return Err(KeyConfigError::WrongKeyLength(n));
            }

            if map.contains_key(&version) {
                return Err(KeyConfigError::DuplicateVersion(n));
            }

            let key = make_key(&key).ok_or(KeyConfigError::WrongKeyLength(n))?;
            map.insert(version, key);
        }

        if !map.contains_key(&active) {
            return Err(KeyConfigError::ActiveVersionMissing);
        }

        Ok(Self { keys: map, active })
    }

    pub fn active_version(&self) -> i16 {
        self.active
    }

    pub fn seal(&self, bot_id: u64, token: &str) -> Result<(Vec<u8>, i16), TokenCryptoError> {
        if token.is_empty() {
            return Err(TokenCryptoError::EmptyToken);
        }

        let key = self.keys.get(&self.active).ok_or(TokenCryptoError::UnknownKeyVersion)?;

        let mut nonce = [0u8; NONCE_LEN];
        SystemRandom::new().fill(&mut nonce).map_err(|_| TokenCryptoError::Random)?;

        let sealed = seal_with_nonce(key, nonce, token.as_bytes(), &bot_id_aad(bot_id))?;
        Ok((sealed, self.active))
    }

    pub fn open(&self, bot_id: u64, version: i16, sealed: &[u8]) -> Result<String, TokenCryptoError> {
        let key = self.keys.get(&version).ok_or(TokenCryptoError::UnknownKeyVersion)?;
        let plaintext = open_sealed(key, sealed, &bot_id_aad(bot_id))?;
        String::from_utf8(plaintext).map_err(|_| TokenCryptoError::DecryptFailed)
    }

    /// A non-empty legacy token wins, because a pod that predates encryption
    /// may have updated only the plaintext column. A failed decryption is
    /// never masked by falling back to anything else.
    pub fn resolve_stored(&self, bot: &StoredToken<'_>) -> Result<String, TokenCryptoError> {
        if let Some(legacy) = bot.legacy_token {
            if !legacy.is_empty() {
                return Ok(legacy.to_owned());
            }
        }

        let (sealed, version) = match (bot.ciphertext, bot.key_version) {
            (Some(sealed), Some(version)) if !sealed.is_empty() => (sealed, version),
            _ => return Err(TokenCryptoError::MissingToken),
        };

        let token = self.open(bot.bot_id, version, sealed)?;
        if token.is_empty() {
            return Err(TokenCryptoError::MissingToken);
        }

        Ok(token)
    }
}

#[cfg(test)]
mod tests;
