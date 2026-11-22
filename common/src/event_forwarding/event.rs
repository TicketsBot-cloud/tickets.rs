use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub const EVENT_KEY: &str = "tickets:events";

#[derive(Deserialize, Serialize, Debug)]
pub struct Event {
    pub bot_id: u64,
    pub is_whitelabel: bool,
    pub shard_id: u16,
    pub event: Box<RawValue>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_legacy_event_with_bot_token() {
        let raw = r#"{"bot_token":"secret.token.value","bot_id":1,"is_whitelabel":true,"shard_id":0,"event":{"t":"X"}}"#;
        let ev: Event = serde_json::from_str(raw).unwrap();
        assert_eq!(ev.bot_id, 1);
        assert!(ev.is_whitelabel);
    }

    #[test]
    fn serialised_event_has_no_token() {
        let ev = Event {
            bot_id: 1,
            is_whitelabel: false,
            shard_id: 0,
            event: RawValue::from_string("{}".to_owned()).unwrap(),
        };
        let out = serde_json::to_string(&ev).unwrap();
        assert!(!out.contains("token"));
    }
}
