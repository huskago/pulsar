use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Permissions(pub i64);

impl Permissions {
    pub const ADMINISTRATOR: i64    = 1 << 0;
    pub const SEND_MESSAGES: i64    = 1 << 1;
    pub const MANAGE_MESSAGES: i64  = 1 << 2;
    pub const MANAGE_CHANNELS: i64  = 1 << 3;
    pub const MANAGE_GUILD: i64     = 1 << 4;
    pub const MANAGE_ROLES: i64     = 1 << 5;
    pub const KICK_MEMBERS: i64     = 1 << 6;
    pub const BAN_MEMBERS: i64      = 1 << 7;
    pub const CREATE_INVITES: i64   = 1 << 8;
    pub const CONNECT_VOICE: i64    = 1 << 9;
    pub const SPEAK: i64            = 1 << 10;
    pub const UPLOAD_FILES: i64     = 1 << 11;

    // Default permissions for @everyone
    pub const DEFAULT: i64
        = Self::SEND_MESSAGES
        | Self::CREATE_INVITES
        | Self::CONNECT_VOICE
        | Self::SPEAK
        | Self::UPLOAD_FILES;

    pub const ALL: i64 = (1 << 12) - 1;

    pub fn new(bits: i64) -> Self {
        Self(bits)
    }

    pub fn has(self, perm: i64) -> bool {
        if self.0 & Self::ADMINISTRATOR != 0 {
            return true;
        }
        self.0 & perm == perm
    }

    pub fn combine(self, other: Permissions) -> Permissions {
        Permissions(self.0 | other.0)
    }
}