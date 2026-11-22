use model::Snowflake;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "tickets:tokenchange";

/// Signals that a whitelabel bot row was created or its credentials changed.
/// It deliberately carries no credentials: consumers re-read the row. Older
/// publishers may still send a `token` field, which is ignored on decode.
#[derive(Serialize, Deserialize, Debug)]
pub struct Payload {
    pub new_id: Snowflake,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_legacy_payload_with_token() {
        let p: Payload =
            serde_json::from_str(r#"{"token":"secret.token.value","new_id":"1234"}"#).unwrap();
        assert_eq!(p.new_id, Snowflake(1234));
    }

    #[test]
    fn decodes_new_payload() {
        let p: Payload = serde_json::from_str(r#"{"new_id":"1234"}"#).unwrap();
        assert_eq!(p.new_id, Snowflake(1234));
    }

    #[test]
    fn rejects_payload_without_id() {
        assert!(serde_json::from_str::<Payload>(r#"{"token":"x"}"#).is_err());
    }

    #[test]
    fn serialised_payload_has_no_token() {
        let out = serde_json::to_string(&Payload {
            new_id: Snowflake(1234),
        })
        .unwrap();
        assert!(!out.contains("token"));
    }
}
