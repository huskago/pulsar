-- channel_keys had no FK to channels, so DEK rows were orphaned on guild/channel deletion.
ALTER TABLE channel_keys
    ADD CONSTRAINT fk_channel_keys_channel
    FOREIGN KEY (channel_id) REFERENCES channels(id) ON DELETE CASCADE;
