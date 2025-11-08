use core::fmt;

use serde::{Deserialize, Serialize};

use model::channel::message::Message;
use model::channel::{Channel, ThreadMember};
use model::guild::{Guild, UnavailableGuild, VoiceState};
use model::interaction::{ApplicationCommand, GuildApplicationCommandPermissions};
use model::stage::StageInstance;
use model::user::{PresenceUpdate, User};

#[derive(Serialize, Debug)]
#[serde(tag = "t", content = "d")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    Ready(super::Ready),
    Resumed(serde_json::Value),
    ApplicationCommandCreate(ApplicationCommand),
    ApplicationCommandUpdate(ApplicationCommand),
    ApplicationCommandDelete(ApplicationCommand),
    ApplicationCommandPermissionsUpdate(GuildApplicationCommandPermissions),
    ChannelCreate(Channel),
    ChannelUpdate(Channel),
    ChannelDelete(Channel),
    ChannelPinsUpdate(super::ChannelPinsUpdate),
    ThreadCreate(Channel),
    ThreadUpdate(Channel),
    ThreadDelete(super::ThreadDelete),
    ThreadListSync(super::ThreadListSync),
    ThreadMemberUpdate(ThreadMember),
    ThreadMembersUpdate(super::ThreadMembersUpdate),
    GuildCreate(Guild),
    GuildUpdate(Guild),
    GuildDelete(UnavailableGuild),
    GuildBanAdd(super::GuildBanAdd),
    GuildBanRemove(super::GuildBanRemove),
    GuildEmojisUpdate(super::GuildEmojisUpdate),
    GuildIntegrationsUpdate(super::GuildIntegrationsUpdate),
    GuildJoinRequestUpdate(super::GuildJoinRequestUpdate),
    GuildJoinRequestDelete(super::GuildJoinRequestDelete),
    GuildMemberAdd(super::GuildMemberAdd),
    GuildMemberRemove(super::GuildMemberRemove),
    GuildMemberUpdate(super::GuildMemberUpdate),
    GuildMembersChunk(super::GuildMembersChunk),
    GuildRoleCreate(super::GuildRoleCreate),
    GuildRoleUpdate(super::GuildRoleUpdate),
    GuildRoleDelete(super::GuildRoleDelete),
    InviteCreate(super::InviteCreate),
    InviteDelete(super::InviteDelete),
    MessageCreate(Message),
    MessageUpdate(Message),
    MessageDelete(super::MessageDelete),
    MessageDeleteBulk(super::MessageDeleteBulk),
    MessageReactionAdd(super::MessageReactionAdd),
    MessageReactionRemove(super::MessageReactionRemove),
    MessageReactionRemoveAll(super::MessageReactionRemoveAll),
    MessageReactionRemoveEmoji(super::MessageReactionRemoveEmoji),
    PresenceUpdate(PresenceUpdate),
    StageInstanceCreate(StageInstance),
    StageInstanceUpdate(StageInstance),
    StageInstanceDelete(StageInstance),
    TypingStart(super::TypingStart),
    UserUpdate(User),
    VoiceChannelStatusUpdate(super::VoiceChannelStatusUpdate),
    VoiceStateUpdate(VoiceState),
    VoiceServerUpdate(super::VoiceServerUpdate),
    WebhookUpdate(super::WebhookUpdate),
    Unknown(serde_json::Value),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Ready(_) => write!(f, "READY"),
            Event::Resumed(_) => write!(f, "RESUMED"),
            Event::ApplicationCommandCreate(_) => write!(f, "APPLICATION_COMMAND_CREATE"),
            Event::ApplicationCommandUpdate(_) => write!(f, "APPLICATION_COMMAND_UPDATE"),
            Event::ApplicationCommandDelete(_) => write!(f, "APPLICATION_COMMAND_DELETE"),
            Event::ApplicationCommandPermissionsUpdate(_) => {
                write!(f, "APPLICATION_COMMAND_PERMISSIONS_UPDATE")
            }
            Event::ChannelCreate(_) => write!(f, "CHANNEL_CREATE"),
            Event::ChannelUpdate(_) => write!(f, "CHANNEL_UPDATE"),
            Event::ChannelDelete(_) => write!(f, "CHANNEL_DELETE"),
            Event::ChannelPinsUpdate(_) => write!(f, "CHANNEL_PINS_UPDATE"),
            Event::ThreadCreate(_) => write!(f, "THREAD_CREATE"),
            Event::ThreadUpdate(_) => write!(f, "THREAD_UPDATE"),
            Event::ThreadDelete(_) => write!(f, "THREAD_DELETE"),
            Event::ThreadListSync(_) => write!(f, "THREAD_LIST_SYNC"),
            Event::ThreadMemberUpdate(_) => write!(f, "THREAD_MEMBER_UPDATE"),
            Event::ThreadMembersUpdate(_) => write!(f, "THREAD_MEMBERS_UPDATE"),
            Event::GuildCreate(_) => write!(f, "GUILD_CREATE"),
            Event::GuildUpdate(_) => write!(f, "GUILD_UPDATE"),
            Event::GuildDelete(_) => write!(f, "GUILD_DELETE"),
            Event::GuildBanAdd(_) => write!(f, "GUILD_BAN_ADD"),
            Event::GuildBanRemove(_) => write!(f, "GUILD_BAN_REMOVE"),
            Event::GuildEmojisUpdate(_) => write!(f, "GUILD_EMOJIS_UPDATE"),
            Event::GuildIntegrationsUpdate(_) => write!(f, "GUILD_INTEGRATIONS_UPDATE"),
            Event::GuildJoinRequestUpdate(_) => write!(f, "GUILD_JOIN_REQUEST_UPDATE"),
            Event::GuildJoinRequestDelete(_) => write!(f, "GUILD_JOIN_REQUEST_DELETE"),
            Event::GuildMemberAdd(_) => write!(f, "GUILD_MEMBER_ADD"),
            Event::GuildMemberRemove(_) => write!(f, "GUILD_MEMBER_REMOVE"),
            Event::GuildMemberUpdate(_) => write!(f, "GUILD_MEMBER_UPDATE"),
            Event::GuildMembersChunk(_) => write!(f, "GUILD_MEMBERS_CHUNK"),
            Event::GuildRoleCreate(_) => write!(f, "GUILD_ROLE_CREATE"),
            Event::GuildRoleUpdate(_) => write!(f, "GUILD_ROLE_UPDATE"),
            Event::GuildRoleDelete(_) => write!(f, "GUILD_ROLE_DELETE"),
            Event::InviteCreate(_) => write!(f, "INVITE_CREATE"),
            Event::InviteDelete(_) => write!(f, "INVITE_DELETE"),
            Event::MessageCreate(_) => write!(f, "MESSAGE_CREATE"),
            Event::MessageUpdate(_) => write!(f, "MESSAGE_UPDATE"),
            Event::MessageDelete(_) => write!(f, "MESSAGE_DELETE"),
            Event::MessageDeleteBulk(_) => write!(f, "MESSAGE_DELETE_BULK"),
            Event::MessageReactionAdd(_) => write!(f, "MESSAGE_REACTION_ADD"),
            Event::MessageReactionRemove(_) => write!(f, "MESSAGE_REACTION_REMOVE"),
            Event::MessageReactionRemoveAll(_) => write!(f, "MESSAGE_REACTION_REMOVE_ALL"),
            Event::MessageReactionRemoveEmoji(_) => write!(f, "MESSAGE_REACTION_REMOVE_EMOJI"),
            Event::PresenceUpdate(_) => write!(f, "PRESENCE_UPDATE"),
            Event::StageInstanceCreate(_) => write!(f, "STAGE_INSTANCE_CREATE"),
            Event::StageInstanceUpdate(_) => write!(f, "STAGE_INSTANCE_UPDATE"),
            Event::StageInstanceDelete(_) => write!(f, "STAGE_INSTANCE_DELETE"),
            Event::TypingStart(_) => write!(f, "TYPING_START"),
            Event::UserUpdate(_) => write!(f, "USER_UPDATE"),
            Event::VoiceChannelStatusUpdate(_) => write!(f, "VOICE_CHANNEL_STATUS_UPDATE"),
            Event::VoiceStateUpdate(_) => write!(f, "VOICE_STATE_UPDATE"),
            Event::VoiceServerUpdate(_) => write!(f, "VOICE_SERVER_UPDATE"),
            Event::WebhookUpdate(_) => write!(f, "WEBHOOK_UPDATE"),
            Event::Unknown(_) => write!(f, "UNKNOWN"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Event {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        use serde_json::json;

        let value = serde_json::Value::deserialize(deserializer)?;
        let obj = value.as_object().ok_or_else(|| {
            Error::custom("expected object for Event")
        })?;

        let tag = obj.get("t").and_then(|v| v.as_str()).unwrap_or("");
        let data = obj.get("d").cloned().unwrap_or(serde_json::Value::Null);

        match tag {
            "READY" => serde_json::from_value::<super::Ready>(data).map(Event::Ready),
            "RESUMED" => Ok(Event::Resumed(data)),
            "APPLICATION_COMMAND_CREATE" => serde_json::from_value::<ApplicationCommand>(data).map(Event::ApplicationCommandCreate),
            "APPLICATION_COMMAND_UPDATE" => serde_json::from_value::<ApplicationCommand>(data).map(Event::ApplicationCommandUpdate),
            "APPLICATION_COMMAND_DELETE" => serde_json::from_value::<ApplicationCommand>(data).map(Event::ApplicationCommandDelete),
            "APPLICATION_COMMAND_PERMISSIONS_UPDATE" => serde_json::from_value::<GuildApplicationCommandPermissions>(data).map(Event::ApplicationCommandPermissionsUpdate),
            "CHANNEL_CREATE" => serde_json::from_value::<Channel>(data).map(Event::ChannelCreate),
            "CHANNEL_UPDATE" => serde_json::from_value::<Channel>(data).map(Event::ChannelUpdate),
            "CHANNEL_DELETE" => serde_json::from_value::<Channel>(data).map(Event::ChannelDelete),
            "CHANNEL_PINS_UPDATE" => serde_json::from_value::<super::ChannelPinsUpdate>(data).map(Event::ChannelPinsUpdate),
            "THREAD_CREATE" => serde_json::from_value::<Channel>(data).map(Event::ThreadCreate),
            "THREAD_UPDATE" => serde_json::from_value::<Channel>(data).map(Event::ThreadUpdate),
            "THREAD_DELETE" => serde_json::from_value::<super::ThreadDelete>(data).map(Event::ThreadDelete),
            "THREAD_LIST_SYNC" => serde_json::from_value::<super::ThreadListSync>(data).map(Event::ThreadListSync),
            "THREAD_MEMBER_UPDATE" => serde_json::from_value::<ThreadMember>(data).map(Event::ThreadMemberUpdate),
            "THREAD_MEMBERS_UPDATE" => serde_json::from_value::<super::ThreadMembersUpdate>(data).map(Event::ThreadMembersUpdate),
            "GUILD_CREATE" => serde_json::from_value::<Guild>(data).map(Event::GuildCreate),
            "GUILD_UPDATE" => serde_json::from_value::<Guild>(data).map(Event::GuildUpdate),
            "GUILD_DELETE" => serde_json::from_value::<UnavailableGuild>(data).map(Event::GuildDelete),
            "GUILD_BAN_ADD" => serde_json::from_value::<super::GuildBanAdd>(data).map(Event::GuildBanAdd),
            "GUILD_BAN_REMOVE" => serde_json::from_value::<super::GuildBanRemove>(data).map(Event::GuildBanRemove),
            "GUILD_EMOJIS_UPDATE" => serde_json::from_value::<super::GuildEmojisUpdate>(data).map(Event::GuildEmojisUpdate),
            "GUILD_INTEGRATIONS_UPDATE" => serde_json::from_value::<super::GuildIntegrationsUpdate>(data).map(Event::GuildIntegrationsUpdate),
            "GUILD_JOIN_REQUEST_UPDATE" => serde_json::from_value::<super::GuildJoinRequestUpdate>(data).map(Event::GuildJoinRequestUpdate),
            "GUILD_JOIN_REQUEST_DELETE" => serde_json::from_value::<super::GuildJoinRequestDelete>(data).map(Event::GuildJoinRequestDelete),
            "GUILD_MEMBER_ADD" => serde_json::from_value::<super::GuildMemberAdd>(data).map(Event::GuildMemberAdd),
            "GUILD_MEMBER_REMOVE" => serde_json::from_value::<super::GuildMemberRemove>(data).map(Event::GuildMemberRemove),
            "GUILD_MEMBER_UPDATE" => serde_json::from_value::<super::GuildMemberUpdate>(data).map(Event::GuildMemberUpdate),
            "GUILD_MEMBERS_CHUNK" => serde_json::from_value::<super::GuildMembersChunk>(data).map(Event::GuildMembersChunk),
            "GUILD_ROLE_CREATE" => serde_json::from_value::<super::GuildRoleCreate>(data).map(Event::GuildRoleCreate),
            "GUILD_ROLE_UPDATE" => serde_json::from_value::<super::GuildRoleUpdate>(data).map(Event::GuildRoleUpdate),
            "GUILD_ROLE_DELETE" => serde_json::from_value::<super::GuildRoleDelete>(data).map(Event::GuildRoleDelete),
            "INVITE_CREATE" => serde_json::from_value::<super::InviteCreate>(data).map(Event::InviteCreate),
            "INVITE_DELETE" => serde_json::from_value::<super::InviteDelete>(data).map(Event::InviteDelete),
            "MESSAGE_CREATE" => serde_json::from_value::<Message>(data).map(Event::MessageCreate),
            "MESSAGE_UPDATE" => serde_json::from_value::<Message>(data).map(Event::MessageUpdate),
            "MESSAGE_DELETE" => serde_json::from_value::<super::MessageDelete>(data).map(Event::MessageDelete),
            "MESSAGE_DELETE_BULK" => serde_json::from_value::<super::MessageDeleteBulk>(data).map(Event::MessageDeleteBulk),
            "MESSAGE_REACTION_ADD" => serde_json::from_value::<super::MessageReactionAdd>(data).map(Event::MessageReactionAdd),
            "MESSAGE_REACTION_REMOVE" => serde_json::from_value::<super::MessageReactionRemove>(data).map(Event::MessageReactionRemove),
            "MESSAGE_REACTION_REMOVE_ALL" => serde_json::from_value::<super::MessageReactionRemoveAll>(data).map(Event::MessageReactionRemoveAll),
            "MESSAGE_REACTION_REMOVE_EMOJI" => serde_json::from_value::<super::MessageReactionRemoveEmoji>(data).map(Event::MessageReactionRemoveEmoji),
            "PRESENCE_UPDATE" => serde_json::from_value::<PresenceUpdate>(data).map(Event::PresenceUpdate),
            "STAGE_INSTANCE_CREATE" => serde_json::from_value::<StageInstance>(data).map(Event::StageInstanceCreate),
            "STAGE_INSTANCE_UPDATE" => serde_json::from_value::<StageInstance>(data).map(Event::StageInstanceUpdate),
            "STAGE_INSTANCE_DELETE" => serde_json::from_value::<StageInstance>(data).map(Event::StageInstanceDelete),
            "TYPING_START" => serde_json::from_value::<super::TypingStart>(data).map(Event::TypingStart),
            "USER_UPDATE" => serde_json::from_value::<User>(data).map(Event::UserUpdate),
            "VOICE_CHANNEL_STATUS_UPDATE" => serde_json::from_value::<super::VoiceChannelStatusUpdate>(data).map(Event::VoiceChannelStatusUpdate),
            "VOICE_STATE_UPDATE" => serde_json::from_value::<VoiceState>(data).map(Event::VoiceStateUpdate),
            "VOICE_SERVER_UPDATE" => serde_json::from_value::<super::VoiceServerUpdate>(data).map(Event::VoiceServerUpdate),
            "WEBHOOK_UPDATE" => serde_json::from_value::<super::WebhookUpdate>(data).map(Event::WebhookUpdate),
            _ => Ok(Event::Unknown(data)),
        }.map_err(|e| Error::custom(format!("Failed to deserialize event '{}': {}", tag, e)))
    }
}
