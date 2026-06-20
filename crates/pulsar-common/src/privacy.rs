use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DmPrivacy {
    Everyone,
    FriendsAndGuilds,
    FriendsOfFriends,
    #[default]
    FriendsOnly,
    Nobody,
}

impl DmPrivacy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Everyone => "everyone",
            Self::FriendsAndGuilds => "friends_and_guilds",
            Self::FriendsOfFriends => "friends_of_friends",
            Self::FriendsOnly => "friends_only",
            Self::Nobody => "nobody",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "everyone" => Self::Everyone,
            "friends_and_guilds" => Self::FriendsAndGuilds,
            "friends_of_friends" => Self::FriendsOfFriends,
            "friends_only" => Self::FriendsOnly,
            "nobody" => Self::Nobody,
            _ => Self::FriendsOnly,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FriendRequestPrivacy {
    #[default]
    Everyone,
    FriendsOfFriends,
    GuildsOnly,
    Nobody,
}

impl FriendRequestPrivacy {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Everyone => "everyone",
            Self::FriendsOfFriends => "friends_of_friends",
            Self::GuildsOnly => "guilds_only",
            Self::Nobody => "nobody",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "everyone" => Self::Everyone,
            "friends_of_friends" => Self::FriendsOfFriends,
            "guilds_only" => Self::GuildsOnly,
            "nobody" => Self::Nobody,
            _ => Self::Everyone,
        }
    }
}