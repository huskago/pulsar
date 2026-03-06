ALTER TABLE channels ALTER COLUMN guild_id DROP NOT NULL;

CREATE TABLE IF NOT EXISTS dm_channels (
    channel_id  BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_dm_channels_user ON dm_channels(user_id);

CREATE INDEX idx_dm_channels_channel ON dm_channels(channel_id);