ALTER TABLE users ADD COLUMN IF NOT EXISTS dm_privacy VARCHAR(20) NOT NULL DEFAULT 'friends_only';
ALTER TABLE users ADD COLUMN IF NOT EXISTS friend_request_privacy VARCHAR(20) NOT NULL DEFAULT 'everyone';

CREATE TABLE IF NOT EXISTS relationships (
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind        VARCHAR(20) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, target_id),
    CHECK (user_id != target_id),
    CHECK (kind IN ('friend', 'blocked', 'pending_outgoing', 'pending_incoming'))
);

CREATE INDEX IF NOT EXISTS idx_relationships_user ON relationships(user_id, kind);
CREATE INDEX IF NOT EXISTS idx_relationships_target ON relationships(target_id, kind);

ALTER TABLE dm_channels ADD COLUMN IF NOT EXISTS is_group BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS group_dm_info (
    channel_id  BIGINT PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE,
    name        VARCHAR(100),
    owner_id    BIGINT NOT NULL REFERENCES users(id),
    icon_url    VARCHAR(512)
);