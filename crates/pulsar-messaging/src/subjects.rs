/// NATS subject naming conventions.
///
/// The hierarchy reflects the Pulsar topology:
///   chat.{guild_id}.{channel_id}  -> messages from a channel
///   presence.{guild_id}           -> presence changes in a guild
///   typing.{guild_id}.{channel_id} -> typing indicators
///
/// NATS supports wildcards:
///   chat.guild_123.*  -> all channels in guild 123
///   chat.>            -> all messages from all guilds
///
/// The '>' is a recursive wildcard (matches everything that follows).
/// The '*' matches a single segment.

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
