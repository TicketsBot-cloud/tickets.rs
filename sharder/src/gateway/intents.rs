#[derive(Copy, Clone)]
#[allow(dead_code)]
pub enum Intents {
    Guilds = 1 << 0,
    GuildMembers = 1 << 1,
    GuildBans = 1 << 2,
    GuildEmojis = 1 << 3,
    GuildIntegrations = 1 << 4,
    GuildWebhooks = 1 << 5,
    GuildInvites = 1 << 6,
    GuildVoiceStates = 1 << 7,
    GuildPresences = 1 << 8,
    GuildMessages = 1 << 9,
    GuildMessageReactions = 1 << 10,
    GuildMessageTyping = 1 << 11,
    DirectMessages = 1 << 12,
    DirectMessageReaction = 1 << 13,
    DirectMessageTyping = 1 << 14,
}

impl Intents {
    pub fn build(intents: Vec<Intents>) -> u64 {
        let mut sum = 0;
        intents.into_iter().for_each(|i| sum |= i as u64);
        sum
    }
}