/// Subject hierarchy:
///   chat.{guild_id}.{channel_id}    channel messages
///   typing.{guild_id}.{channel_id}  typing indicators
///   presence.{guild_id}             presence updates
///
/// Wildcards: `*` matches one segment, `>` matches the rest of the subject.

pub fn chat_channel(guild_id: &str, channel_id: &str) -> String {
    format!("chat.{}.{}", guild_id, channel_id)
}

pub fn chat_guild(guild_id: &str) -> String {
    format!("chat.{}.>", guild_id)
}

pub fn presence_guild(guild_id: &str) -> String {
    format!("presence.{}", guild_id)
}

pub fn typing_channel(guild_id: &str, channel_id: &str) -> String {
    format!("typing.{}.{}", guild_id, channel_id)
}
