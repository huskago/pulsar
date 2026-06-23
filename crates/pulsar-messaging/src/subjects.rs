//! Subject hierarchy:
//!   chat.{guild_id}.{channel_id}    channel messages
//!   typing.{guild_id}.{channel_id}  typing indicators
//!   presence.{guild_id}             presence updates
//!
//! Wildcards: `*` matches one segment, `>` matches the rest of the subject.
//! All ID parameters are i64 so callers cannot accidentally inject NATS wildcards.

pub fn chat_channel(guild_id: i64, channel_id: i64) -> String {
    format!("chat.{}.{}", guild_id, channel_id)
}

pub fn chat_guild(guild_id: i64) -> String {
    format!("chat.{}.>", guild_id)
}

pub fn presence_guild(guild_id: i64) -> String {
    format!("presence.{}", guild_id)
}

pub fn typing_channel(guild_id: i64, channel_id: i64) -> String {
    format!("typing.{}.{}", guild_id, channel_id)
}

pub fn typing_guild(guild_id: i64) -> String {
    format!("typing.{}.>", guild_id)
}

pub fn dm_user(user_id: i64) -> String {
    format!("dm.user.{}", user_id)
}
