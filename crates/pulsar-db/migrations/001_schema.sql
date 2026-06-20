-- Messages are stored in ScyllaDB, not here.

CREATE OR REPLACE FUNCTION update_updated_at()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE users (
    id                      BIGINT PRIMARY KEY,
    username                VARCHAR(32)  UNIQUE NOT NULL,
    email                   VARCHAR(255) UNIQUE NOT NULL,
    password_hash           TEXT NOT NULL,
    avatar_url              TEXT,
    status                  VARCHAR(20)  NOT NULL DEFAULT 'offline',
    dm_privacy              VARCHAR(20)  NOT NULL DEFAULT 'friends_only',
    friend_request_privacy  VARCHAR(20)  NOT NULL DEFAULT 'everyone',
    created_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email    ON users (email);
CREATE INDEX idx_users_username ON users (username);

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TABLE sessions (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_name TEXT        NOT NULL,
    token_hash  BYTEA       NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX sessions_user_id_idx    ON sessions (user_id);
CREATE INDEX sessions_token_hash_idx ON sessions (token_hash);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);

CREATE TABLE relationships (
    user_id    BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_id  BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind       VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, target_id),
    CHECK (user_id != target_id),
    CHECK (kind IN ('friend', 'blocked', 'pending_outgoing', 'pending_incoming'))
);

CREATE INDEX idx_relationships_user   ON relationships (user_id,   kind);
CREATE INDEX idx_relationships_target ON relationships (target_id, kind);

CREATE TABLE guilds (
    id         BIGINT       PRIMARY KEY,
    name       VARCHAR(100) NOT NULL,
    icon_url   TEXT,
    owner_id   BIGINT       NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE guild_members (
    guild_id  BIGINT      NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    user_id   BIGINT      NOT NULL REFERENCES users(id)  ON DELETE CASCADE,
    nickname  VARCHAR(32),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX idx_guild_members_user ON guild_members (user_id);

-- guild_id is NULL for DM and group DM channels
CREATE TABLE channels (
    id         BIGINT       PRIMARY KEY,
    guild_id   BIGINT       REFERENCES guilds(id) ON DELETE CASCADE,
    name       VARCHAR(100) NOT NULL,
    kind       VARCHAR(20)  NOT NULL DEFAULT 'text',
    position   INT          NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_channels_guild ON channels (guild_id);

CREATE TABLE channel_keys (
    channel_id BIGINT      PRIMARY KEY,
    sealed_dek BYTEA       NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE dm_channels (
    channel_id      BIGINT      NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id         BIGINT      NOT NULL REFERENCES users(id)    ON DELETE CASCADE,
    is_group        BOOLEAN     NOT NULL DEFAULT FALSE,
    last_message_at TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX idx_dm_channels_user    ON dm_channels (user_id);
CREATE INDEX idx_dm_channels_channel ON dm_channels (channel_id);

CREATE TABLE group_dm_info (
    channel_id BIGINT        PRIMARY KEY REFERENCES channels(id) ON DELETE CASCADE,
    name       VARCHAR(100),
    owner_id   BIGINT        NOT NULL REFERENCES users(id),
    icon_url   VARCHAR(512)
);

CREATE TABLE invites (
    code       VARCHAR(10)  PRIMARY KEY,
    guild_id   BIGINT       NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    creator_id BIGINT       NOT NULL REFERENCES users(id),
    max_uses   INT,
    uses       INT          NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invites_guild ON invites (guild_id);

CREATE TABLE roles (
    id          BIGINT       PRIMARY KEY,
    guild_id    BIGINT       NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    name        VARCHAR(100) NOT NULL,
    color       INTEGER      NOT NULL DEFAULT 0,
    position    INTEGER      NOT NULL DEFAULT 0,
    permissions BIGINT       NOT NULL DEFAULT 0,
    is_default  BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_roles_guild ON roles (guild_id);

CREATE TABLE member_roles (
    guild_id    BIGINT      NOT NULL,
    user_id     BIGINT      NOT NULL,
    role_id     BIGINT      NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id, role_id),
    FOREIGN KEY (guild_id, user_id) REFERENCES guild_members(guild_id, user_id) ON DELETE CASCADE
);

CREATE INDEX idx_member_roles_user ON member_roles (guild_id, user_id);

-- File content is encrypted in MinIO, this table holds metadata only.
CREATE TABLE attachments (
    id           BIGINT        PRIMARY KEY,
    message_id   BIGINT        NOT NULL,
    filename     VARCHAR(255)  NOT NULL,
    content_type VARCHAR(127)  NOT NULL,
    size         BIGINT        NOT NULL,
    storage_key  VARCHAR(512)  NOT NULL,
    url          VARCHAR(1024) NOT NULL,
    created_at   TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_attachments_message ON attachments (message_id);
