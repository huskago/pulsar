CREATE TABLE IF NOT EXISTS attachments
(
    id           BIGINT PRIMARY KEY,
    message_id   BIGINT        NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    filename     VARCHAR(255)  NOT NULL,
    content_type VARCHAR(127)  NOT NULL,
    size         BIGINT        NOT NULL,
    storage_key  VARCHAR(512)  NOT NULL,
    url          VARCHAR(1024) NOT NULL,
    created_at   TIMESTAMPTZ   NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_attachments_message ON attachments (message_id);