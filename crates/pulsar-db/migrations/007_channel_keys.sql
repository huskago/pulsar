CREATE TABLE channel_keys (
    channel_id  BIGINT PRIMARY KEY,
    sealed_dek  BYTEA NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
