CREATE TABLE IF NOT EXISTS invites
(
    code       VARCHAR(10) PRIMARY KEY,
    guild_id   BIGINT      NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    creator_id BIGINT      NOT NULL REFERENCES users (id),
    max_uses   INT,
    uses       INT         NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invites_guild ON invites (guild_id);