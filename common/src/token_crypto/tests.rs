use super::*;
use serde::Deserialize;
use std::convert::TryInto;

#[derive(Deserialize)]
struct Vector {
    name: String,
    key_version: i16,
    key_b64: String,
    nonce_hex: String,
    bot_id: u64,
    plaintext: String,
    sealed_hex: String,
}

fn vectors() -> Vec<Vector> {
    let v: Vec<Vector> = serde_json::from_str(include_str!("testdata/token_vectors.json")).unwrap();
    assert!(!v.is_empty());
    v
}

fn hex_decode(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn b64(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

fn key(b: u8) -> Vec<u8> {
    vec![b; 32]
}

fn ring(active: &str) -> TokenKeyring {
    let cfg = format!("1:{},2:{}", b64(&key(0x11)), b64(&key(0x22)));
    TokenKeyring::parse(&cfg, active).unwrap()
}

#[test]
fn vectors_reproduce_and_open() {
    for v in vectors() {
        let raw_key = BASE64.decode(&v.key_b64).unwrap();
        let k = make_key(&raw_key).unwrap();
        let nonce: [u8; 12] = hex_decode(&v.nonce_hex).try_into().unwrap();
        let want = hex_decode(&v.sealed_hex);

        let got = seal_with_nonce(&k, nonce, v.plaintext.as_bytes(), &bot_id_aad(v.bot_id)).unwrap();
        assert_eq!(got, want, "{}", v.name);

        let opened = open_sealed(&k, &want, &bot_id_aad(v.bot_id)).unwrap();
        assert_eq!(opened, v.plaintext.as_bytes(), "{}", v.name);

        let cfg = format!("{}:{}", v.key_version, v.key_b64);
        let ring = TokenKeyring::parse(&cfg, &v.key_version.to_string()).unwrap();
        assert_eq!(ring.open(v.bot_id, v.key_version, &want).unwrap(), v.plaintext, "{}", v.name);
    }
}

#[test]
fn bot_id_aad_is_big_endian() {
    assert_eq!(bot_id_aad(0), [0; 8]);
    assert_eq!(bot_id_aad(1), [0, 0, 0, 0, 0, 0, 0, 1]);
    assert_eq!(bot_id_aad(0x0102030405060708), [1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(bot_id_aad(u64::MAX), [0xff; 8]);
}

#[test]
fn round_trip() {
    let long = "x".repeat(4096);
    let cases: Vec<(u64, &str)> = vec![
        (123456789, "fake-token"),
        (0, "fake-token"),
        (u64::MAX, "fake-token"),
        (7, long.as_str()),
        (7, "x"),
        (7, "fake-t\u{f6}ken"),
    ];

    let ring = ring("2");
    for (bot_id, token) in cases {
        let (sealed, version) = ring.seal(bot_id, token).unwrap();
        assert_eq!(version, 2);
        assert!(sealed.len() >= MIN_SEALED_LEN);
        assert_eq!(ring.open(bot_id, version, &sealed).unwrap(), token);
    }

    let (a, _) = ring.seal(1, "fake-token").unwrap();
    let (b, _) = ring.seal(1, "fake-token").unwrap();
    assert_ne!(a, b);

    assert_eq!(ring.seal(1, "").unwrap_err(), TokenCryptoError::EmptyToken);
}

#[test]
fn wrong_bot_id_fails() {
    let ring = ring("1");
    let (sealed, version) = ring.seal(100, "fake-token").unwrap();
    assert_eq!(ring.open(101, version, &sealed).unwrap_err(), TokenCryptoError::DecryptFailed);
    assert_eq!(ring.open(0, version, &sealed).unwrap_err(), TokenCryptoError::DecryptFailed);
}

#[test]
fn tampering_fails() {
    let ring = ring("1");
    let (sealed, version) = ring.seal(5, "fake-token-for-tamper-test").unwrap();

    let indexes = [
        0,
        NONCE_LEN - 1,
        NONCE_LEN,
        sealed.len() - TAG_LEN - 1,
        sealed.len() - TAG_LEN,
        sealed.len() - 1,
    ];
    for i in indexes {
        let mut m = sealed.clone();
        m[i] ^= 0x01;
        assert_eq!(ring.open(5, version, &m).unwrap_err(), TokenCryptoError::DecryptFailed, "index {}", i);
    }

    let truncated = &sealed[..sealed.len() - 1];
    assert_eq!(ring.open(5, version, truncated).unwrap_err(), TokenCryptoError::DecryptFailed);
}

#[test]
fn sealed_lengths() {
    let ring = ring("1");
    for len in [0usize, 11, 12, 27] {
        let err = ring.open(1, 1, &vec![0u8; len]).unwrap_err();
        assert_eq!(err, TokenCryptoError::SealedTooShort, "len {}", len);
    }

    for len in [28usize, 29] {
        let err = ring.open(1, 1, &vec![0u8; len]).unwrap_err();
        assert_eq!(err, TokenCryptoError::DecryptFailed, "len {}", len);
    }

    let k = make_key(&key(0x11)).unwrap();
    let sealed = seal_with_nonce(&k, [0; 12], b"", &bot_id_aad(1)).unwrap();
    assert_eq!(sealed.len(), MIN_SEALED_LEN);
    assert_eq!(open_sealed(&k, &sealed, &bot_id_aad(1)).unwrap(), Vec::<u8>::new());
}

#[test]
fn parse_failures_never_echo_input() {
    let good = b64(&key(0x33));
    let short = b64(&vec![0x44u8; 31]);
    let url_safe = b64(&vec![0xfbu8; 32]).replace('+', "-").replace('/', "_");
    let non_canonical = format!("{}B=", &good[..good.len() - 2]);
    let newline = format!("{}\n{}", &good[..20], &good[20..]);

    let cases: Vec<(&str, String, &str, KeyConfigError)> = vec![
        ("empty keys", String::new(), "1", KeyConfigError::Empty),
        ("whitespace keys", "   ".into(), "1", KeyConfigError::Empty),
        ("empty active", format!("1:{good}"), "", KeyConfigError::InvalidActiveVersion),
        ("missing colon", format!("1{good}"), "1", KeyConfigError::MalformedEntry(1)),
        ("empty entry", format!("1:{good},"), "1", KeyConfigError::MalformedEntry(2)),
        ("empty key part", "1:".into(), "1", KeyConfigError::WrongKeyLength(1)),
        ("bad base64", "1:not*base64*not*base64*not*base64*not*base64!!".into(), "1", KeyConfigError::InvalidBase64(1)),
        ("newline in base64", format!("1:{newline}"), "1", KeyConfigError::InvalidBase64(1)),
        ("url safe alphabet", format!("1:{url_safe}"), "1", KeyConfigError::InvalidBase64(1)),
        ("non canonical bits", format!("1:{non_canonical}"), "1", KeyConfigError::InvalidBase64(1)),
        ("length 31", format!("1:{short}"), "1", KeyConfigError::WrongKeyLength(1)),
        ("length 16", format!("1:{}", b64(&key(1)[..16])), "1", KeyConfigError::WrongKeyLength(1)),
        ("length 33", format!("1:{}", b64(&vec![1u8; 33])), "1", KeyConfigError::WrongKeyLength(1)),
        ("duplicate", format!("1:{good},1:{}", b64(&key(0x55))), "1", KeyConfigError::DuplicateVersion(2)),
        ("version zero", format!("0:{good}"), "0", KeyConfigError::InvalidActiveVersion),
        ("version too large", format!("32768:{good}"), "32768", KeyConfigError::InvalidActiveVersion),
        ("version negative", format!("-1:{good}"), "1", KeyConfigError::InvalidVersion(1)),
        ("version leading zero", format!("01:{good}"), "1", KeyConfigError::InvalidVersion(1)),
        ("version not numeric", format!("a:{good}"), "1", KeyConfigError::InvalidVersion(1)),
        ("version overflow", format!("99999999999999999999:{good}"), "1", KeyConfigError::InvalidVersion(1)),
        ("active absent", format!("1:{good}"), "2", KeyConfigError::ActiveVersionMissing),
        ("active not numeric", format!("1:{good}"), "latest", KeyConfigError::InvalidActiveVersion),
        ("active zero", format!("1:{good}"), "0", KeyConfigError::InvalidActiveVersion),
    ];

    for (name, keys, active, want) in cases {
        let err = TokenKeyring::parse(&keys, active).unwrap_err();
        assert_eq!(err, want, "{}", name);

        for rendered in [err.to_string(), format!("{:?}", err)] {
            assert!(!rendered.contains(&good), "{}", name);
            assert!(!rendered.contains(&short), "{}", name);
            if keys.len() > 8 {
                assert!(!rendered.contains(&keys), "{}", name);
                assert!(!rendered.contains(&keys[2..]), "{}", name);
            }
        }
    }
}

#[test]
fn parse_success() {
    let (a, b) = (b64(&key(1)), b64(&key(2)));
    let cases = vec![
        (format!("1:{a}"), "1", 1i16),
        (format!("1:{a},2:{b}"), "1", 1),
        (format!("1:{a},2:{b}"), "2", 2),
        (format!("32767:{a}"), "32767", 32767),
        (format!(" 1:{a} , 2:{b} "), " 2 ", 2),
    ];

    for (keys, active, want) in cases {
        assert_eq!(TokenKeyring::parse(&keys, active).unwrap().active_version(), want);
    }
}

#[test]
fn rotation() {
    let old = ring("1");
    let (sealed_old, old_version) = old.seal(9, "fake-token").unwrap();
    assert_eq!(old_version, 1);

    let rotated = ring("2");
    assert_eq!(rotated.active_version(), 2);
    assert_eq!(rotated.open(9, old_version, &sealed_old).unwrap(), "fake-token");

    let (sealed_new, new_version) = rotated.seal(9, "fake-token").unwrap();
    assert_eq!(new_version, 2);
    assert_eq!(rotated.open(9, new_version, &sealed_new).unwrap(), "fake-token");
    assert_eq!(rotated.open(9, 1, &sealed_new).unwrap_err(), TokenCryptoError::DecryptFailed);
}

#[test]
fn unknown_version() {
    let ring = ring("1");
    let (sealed, _) = ring.seal(1, "fake-token").unwrap();
    for version in [-1i16, 0, 3, 32767] {
        assert_eq!(ring.open(1, version, &sealed).unwrap_err(), TokenCryptoError::UnknownKeyVersion);
    }
}

#[test]
fn resolve_stored_precedence_and_failure() {
    let ring = ring("1");
    let (sealed, version) = ring.seal(77, "sealed-token").unwrap();
    let mut corrupt = sealed.clone();
    *corrupt.last_mut().unwrap() ^= 0x01;
    let short = [0u8; 5];

    struct Case<'a> {
        name: &'static str,
        legacy: Option<&'a str>,
        ciphertext: Option<&'a [u8]>,
        version: Option<i16>,
        want: Result<&'static str, TokenCryptoError>,
    }

    let cases = vec![
        Case { name: "legacy wins", legacy: Some("legacy-token"), ciphertext: Some(&sealed), version: Some(version), want: Ok("legacy-token") },
        Case { name: "legacy wins over corrupt", legacy: Some("legacy-token"), ciphertext: Some(&corrupt), version: Some(version), want: Ok("legacy-token") },
        Case { name: "legacy only", legacy: Some("legacy-token"), ciphertext: None, version: None, want: Ok("legacy-token") },
        Case { name: "sealed when legacy none", legacy: None, ciphertext: Some(&sealed), version: Some(version), want: Ok("sealed-token") },
        Case { name: "sealed when legacy empty", legacy: Some(""), ciphertext: Some(&sealed), version: Some(version), want: Ok("sealed-token") },
        Case { name: "corrupt is error", legacy: None, ciphertext: Some(&corrupt), version: Some(version), want: Err(TokenCryptoError::DecryptFailed) },
        Case { name: "corrupt with empty legacy is error", legacy: Some(""), ciphertext: Some(&corrupt), version: Some(version), want: Err(TokenCryptoError::DecryptFailed) },
        Case { name: "unknown version", legacy: None, ciphertext: Some(&sealed), version: Some(9), want: Err(TokenCryptoError::UnknownKeyVersion) },
        Case { name: "short sealed", legacy: None, ciphertext: Some(&short), version: Some(version), want: Err(TokenCryptoError::SealedTooShort) },
        Case { name: "sealed without version", legacy: None, ciphertext: Some(&sealed), version: None, want: Err(TokenCryptoError::MissingToken) },
        Case { name: "version without sealed", legacy: None, ciphertext: None, version: Some(version), want: Err(TokenCryptoError::MissingToken) },
        Case { name: "empty sealed", legacy: None, ciphertext: Some(&[]), version: Some(version), want: Err(TokenCryptoError::MissingToken) },
        Case { name: "nothing stored", legacy: None, ciphertext: None, version: None, want: Err(TokenCryptoError::MissingToken) },
        Case { name: "empty legacy only", legacy: Some(""), ciphertext: None, version: None, want: Err(TokenCryptoError::MissingToken) },
    ];

    for c in cases {
        let stored = StoredToken { bot_id: 77, legacy_token: c.legacy, ciphertext: c.ciphertext, key_version: c.version };
        let got = ring.resolve_stored(&stored);
        assert_eq!(got, c.want.map(String::from), "{}", c.name);
    }

    let wrong = StoredToken { bot_id: 78, legacy_token: None, ciphertext: Some(&sealed), key_version: Some(version) };
    assert_eq!(ring.resolve_stored(&wrong).unwrap_err(), TokenCryptoError::DecryptFailed);
}

#[test]
fn debug_output_is_redacted() {
    let k = key(0x66);
    let cfg = format!("1:{}", b64(&k));
    let ring = TokenKeyring::parse(&cfg, "1").unwrap();

    for rendered in [format!("{:?}", ring), format!("{:#?}", ring)] {
        assert!(!rendered.contains(&b64(&k)));
        assert!(!rendered.contains("102"));
    }

    let stored = StoredToken { bot_id: 1, legacy_token: Some("super-secret-fake"), ciphertext: Some(&[1, 2, 3]), key_version: Some(1) };
    assert!(!format!("{:?}", stored).contains("super-secret-fake"));

    let err = ring.open(1, 1, &[0u8; 40]).unwrap_err();
    assert!(!format!("{} {:?}", err, err).contains("super-secret-fake"));
}
