use super::Opcode;
use crate::gateway::ShardInfo;
use serde::Serialize;
use std::fmt;

#[derive(Serialize)]
pub struct Identify {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    pub data: IdentifyData,
}

impl fmt::Debug for Identify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identify")
            .field("opcode", &self.opcode)
            .field("data", &self.data)
            .finish()
    }
}

impl fmt::Debug for IdentifyData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentifyData")
            .field("token", &"<redacted>")
            .field("properties", &self.properties)
            .field("compress", &self.compress)
            .field("large_threshold", &self.large_threshold)
            .field("shard_info", &self.shard_info)
            .field("presence", &self.presence)
            .field("guild_subscriptions", &self.guild_subscriptions)
            .field("intents", &self.intents)
            .finish()
    }
}

impl Identify {
    /// large_threshold must be between 50 and 250 inclusive, if Some
    pub fn new(
        token: String,
        large_threshold: Option<i32>,
        shard_info: ShardInfo,
        presence: Option<model::user::StatusUpdate>,
        intents: u64,
    ) -> Identify {
        if let Some(large_threshold) = large_threshold {
            if !(50..=250).contains(&large_threshold) {
                panic!("large_threshold must be between 50 and 250 inclusive");
            }
        }

        Identify {
            opcode: Opcode::Identify,
            data: IdentifyData {
                token,
                properties: ConnectionProperties::new(),
                compress: Some(cfg!(feature = "compression")),
                large_threshold,
                shard_info,
                presence,
                guild_subscriptions: None,
                intents,
            },
        }
    }
}

#[derive(Serialize)]
pub struct IdentifyData {
    pub token: String,

    pub properties: ConnectionProperties,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_threshold: Option<i32>,

    #[serde(rename = "shard")]
    pub shard_info: ShardInfo,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<model::user::StatusUpdate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_subscriptions: Option<bool>,

    pub intents: u64,
}

#[derive(Serialize, Debug)]
pub struct ConnectionProperties {
    #[serde(rename = "$os")]
    pub os: String,

    #[serde(rename = "browser")]
    pub browser: String,

    #[serde(rename = "device")]
    pub device: String,
}

const LIBRARY_NAME: &str = "tickets.rs";

impl ConnectionProperties {
    pub fn new() -> ConnectionProperties {
        ConnectionProperties {
            os: std::env::consts::OS.to_owned(),
            browser: LIBRARY_NAME.to_owned(),
            device: LIBRARY_NAME.to_owned(),
        }
    }
}

impl Default for ConnectionProperties {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_token() {
        let identify = Identify::new(
            "super.secret.token".to_owned(),
            None,
            ShardInfo::new(0, 1),
            None,
            0,
        );

        let out = format!("{:?} {:#?}", identify, identify.data);
        assert!(!out.contains("super.secret.token"));
        assert!(out.contains("<redacted>"));
    }
}
