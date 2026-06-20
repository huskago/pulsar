use pulsar_common::{error::AppError, privacy::{DmPrivacy, FriendRequestPrivacy}};
use pulsar_db::repo::{relationships, users};
use sqlx::PgPool;

pub async fn can_dm(
    db: &PgPool,
    sender_id: i64,
    target_id: i64,
) -> Result<bool, AppError> {
    if relationships::is_blocked(db, target_id, sender_id).await? {
        return Ok(false);
    }

    let settings = users::get_settings(db, target_id).await?;
    let privacy = DmPrivacy::from_db(&settings.dm_privacy);

    match privacy {
        DmPrivacy::Everyone => Ok(true),
        DmPrivacy::FriendsAndGuilds => {
            let friend = relationships::is_friend(db, sender_id, target_id).await?;
            if friend { return Ok(true); }
            relationships::share_guild(db, sender_id, target_id).await
        }
        DmPrivacy::FriendsOfFriends => {
            let friend = relationships::is_friend(db, sender_id, target_id).await?;
            if friend { return Ok(true); }
            relationships::has_mutual_friends(db, sender_id, target_id).await
        }
        DmPrivacy::FriendsOnly => {
            relationships::is_friend(db, sender_id, target_id).await
        }
        DmPrivacy::Nobody => Ok(false),
    }
}

pub async fn can_send_friend_request(
    db: &PgPool,
    sender_id: i64,
    target_id: i64,
) -> Result<bool, AppError> {
    if relationships::is_blocked(db, sender_id, target_id).await? {
        return Ok(false);
    }

    let settings = users::get_settings(db, target_id).await?;
    let privacy = FriendRequestPrivacy::from_db(&settings.friend_request_privacy);

    match privacy {
        FriendRequestPrivacy::Everyone => Ok(true),
        FriendRequestPrivacy::FriendsOfFriends => {
            relationships::has_mutual_friends(db, sender_id, target_id).await
        }
        FriendRequestPrivacy::GuildsOnly => {
            relationships::share_guild(db, sender_id, target_id).await
        }
        FriendRequestPrivacy::Nobody => Ok(false),
    }
}