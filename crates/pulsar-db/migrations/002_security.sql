-- Track the latest JTI per session so we can blocklist on explicit revoke.
ALTER TABLE sessions ADD COLUMN current_jti TEXT;
CREATE INDEX idx_sessions_current_jti ON sessions (current_jti) WHERE current_jti IS NOT NULL;

-- Tracks files uploaded but not yet linked to a message.
-- Gateway verifies ownership before accepting an attachment in SendMessage.
CREATE TABLE pending_attachments (
    user_id     BIGINT       NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id  BIGINT       NOT NULL,
    storage_key VARCHAR(512) NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, storage_key)
);
CREATE INDEX idx_pending_attachments_key ON pending_attachments (storage_key);
