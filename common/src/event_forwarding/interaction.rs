use model::interaction::InteractionType;
use serde::Serialize;
use serde_json::value::RawValue;

pub const COMMAND_KEY: &str = "tickets:commands";

#[derive(Serialize, Debug)]
pub struct ForwardedInteraction {
    pub bot_id: u64,
    pub is_whitelabel: bool,
    pub interaction_type: InteractionType,
    pub data: Box<RawValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::interaction::InteractionType;

    #[test]
    fn serialised_interaction_has_no_token() {
        let fwd = ForwardedInteraction {
            bot_id: 1,
            is_whitelabel: true,
            interaction_type: InteractionType::ApplicationCommand,
            data: RawValue::from_string("{}".to_owned()).unwrap(),
        };
        let out = serde_json::to_string(&fwd).unwrap();
        assert!(!out.contains("token"));
    }
}
