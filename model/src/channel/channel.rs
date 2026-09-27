use serde::{Deserialize, Serialize};

use super::{ChannelType, PermissionOverwrite};
use crate::channel::{ThreadMember, ThreadMetadata, VideoQualityMode};
use crate::user::User;
use crate::Snowflake;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Channel {
    #[serde(skip_serializing)]
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    #[serde(skip_serializing)]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "Snowflake::serialize_option_to_int"
    )]
    pub last_message_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipients: Option<Vec<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<VideoQualityMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_metadata: Option<ThreadMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_member: Option<ThreadMember>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guild::Guild;
    use serde_json::{json, Value};

    const OBFUSCATED: u64 = 1 << 17;

    const OPTIONAL_FIELDS: [&str; 5] = [
        "permission_overwrites",
        "thread_metadata",
        "video_quality_mode",
        "bitrate",
        "user_limit",
    ];

    fn obfuscated_channel() -> Value {
        json!({
            "id": "222",
            "type": 0,
            "guild_id": "111",
            "position": 3,
            "parent_id": "333",
            "name": "___hidden___",
            "flags": 131072,
            "permission_overwrites": [{"id": "111", "type": 0, "allow": "0", "deny": "1024"}],
            "topic": null,
            "nsfw": null,
            "last_message_id": null,
            "rate_limit_per_user": null,
            "thread_metadata": null,
            "video_quality_mode": null,
            "bitrate": null,
            "user_limit": null
        })
    }

    fn variants() -> Vec<Value> {
        let mut variants = vec![obfuscated_channel()];

        for field in OPTIONAL_FIELDS {
            let mut null = obfuscated_channel();
            null[field] = Value::Null;
            variants.push(null);

            let mut absent = obfuscated_channel();
            absent.as_object_mut().unwrap().remove(field);
            variants.push(absent);
        }

        variants
    }

    fn guild_create(channel: Value) -> Value {
        json!({
            "id": "111",
            "name": "guild",
            "icon": null,
            "owner_id": "444",
            "region": "deprecated",
            "afk_timeout": 300,
            "verification_level": 0,
            "default_message_notifications": 0,
            "explicit_content_filter": 0,
            "roles": [],
            "features": [],
            "mfa_level": 0,
            "premium_tier": 0,
            "preferred_locale": "en-US",
            "channels": [channel]
        })
    }

    fn assert_keeps_flags(channel: &Channel) {
        assert_eq!(channel.flags, Some(OBFUSCATED));
        let encoded = serde_json::to_string(channel).unwrap();
        assert!(encoded.contains(r#""flags":131072"#), "{}", encoded);
    }

    #[test]
    fn test_obfuscated_channel_keeps_flags() {
        for raw in variants() {
            let channel: Channel =
                serde_json::from_value(raw.clone()).unwrap_or_else(|e| panic!("{}: {}", e, raw));
            assert_keeps_flags(&channel);
        }
    }

    #[test]
    fn test_obfuscated_guild_channel_keeps_flags() {
        for mut raw in variants() {
            raw.as_object_mut().unwrap().remove("guild_id");

            let guild: Guild = serde_json::from_value(guild_create(raw.clone()))
                .unwrap_or_else(|e| panic!("{}: {}", e, raw));
            let channels = guild.channels.unwrap();
            assert_eq!(channels.len(), 1);
            assert_keeps_flags(&channels[0]);
        }
    }

    #[test]
    fn test_missing_flags_are_not_serialized() {
        let mut null = obfuscated_channel();
        null["flags"] = Value::Null;

        let mut absent = obfuscated_channel();
        absent.as_object_mut().unwrap().remove("flags");

        for raw in [null, absent] {
            let channel: Channel = serde_json::from_value(raw).unwrap();
            assert_eq!(channel.flags, None);
            assert!(!serde_json::to_string(&channel).unwrap().contains("flags"));
        }
    }
}
