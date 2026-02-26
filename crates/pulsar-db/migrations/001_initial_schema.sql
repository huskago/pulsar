CREATE TABLE IF NOT EXISTS users
(
    id            BIGINT PRIMARY KEY,
    username      VARCHAR(32) UNIQUE  NOT NULL,
    email         VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT                NOT NULL,
    avatar_url    TEXT,
    status        VARCHAR(20)         NOT NULL DEFAULT 'offline',
    created_at    TIMESTAMPTZ         NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ         NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users (email);
CREATE INDEX idx_users_username ON users (username);

CREATE TABLE IF NOT EXISTS guilds
(
    id         BIGINT PRIMARY KEY,
    name       VARCHAR(100) NOT NULL,
    icon_url   TEXT,
    owner_id   BIGINT       NOT NULL REFERENCES users (id),
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS guild_members
(
    guild_id  BIGINT      NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    user_id   BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    nickname  VARCHAR(32),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX idx_guild_members_user ON guild_members (user_id);

CREATE TABLE IF NOT EXISTS channels
(
    id         BIGINT PRIMARY KEY,
    guild_id   BIGINT       NOT NULL REFERENCES guilds (id) ON DELETE CASCADE,
    name       VARCHAR(100) NOT NULL,
    kind       VARCHAR(20)  NOT NULL DEFAULT 'text',
    position   INT          NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_channels_guild ON channels (guild_id);

CREATE TABLE IF NOT EXISTS messages
(
    id         BIGINT PRIMARY KEY,
    channel_id BIGINT      NOT NULL REFERENCES channels (id) ON DELETE CASCADE,
    author_id  BIGINT      NOT NULL REFERENCES users (id),
    content    TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    edited_at  TIMESTAMPTZ
);

CREATE INDEX idx_messages_channel_time ON messages (channel_id, id DESC);

CREATE OR REPLACE FUNCTION update_updated_at()
    RETURNS TRIGGER AS
$$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_updated_at
    BEFORE UPDATE
    ON users
    FOR EACH ROW
EXECUTE FUNCTION update_updated_at();