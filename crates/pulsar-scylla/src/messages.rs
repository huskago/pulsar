use crate::ScyllaClient;

#[derive(Debug, Clone)]
pub struct ScyllaMessage {
    pub channel_id: i64,
    pub message_id: i64,
    pub author_id: i64,
    pub content: Vec<u8>,
    pub edited_at: Option<i64>,
}

pub async fn insert(client: &ScyllaClient, msg: &ScyllaMessage) -> anyhow::Result<()> {
    client.session().query_unpaged(
        "INSERT INTO pulsar.messages (channel_id, message_id, author_id, content, edited_at) VALUES (?, ?, ?, ?, ?)",
        (msg.channel_id, msg.message_id, msg.author_id, &msg.content, msg.edited_at),
    ).await?;
    Ok(())
}

pub async fn find_by_channel(
    client: &ScyllaClient,
    channel_id: i64,
    limit: i32,
    before: Option<i64>,
) -> anyhow::Result<Vec<ScyllaMessage>> {
    let result = if let Some(before_id) = before {
        client.session().query_unpaged(
            "SELECT channel_id, message_id, author_id, content, edited_at FROM pulsar.messages WHERE channel_id = ? AND message_id < ? LIMIT ?",
            (channel_id, before_id, limit),
        ).await?
    } else {
        client.session().query_unpaged(
            "SELECT channel_id, message_id, author_id, content, edited_at FROM pulsar.messages WHERE channel_id = ? LIMIT ?",
            (channel_id, limit),
        ).await?
    };

    let mut messages = Vec::new();
    for row in result.into_rows_result()?.rows::<(i64, i64, i64, Vec<u8>, Option<i64>)>()? {
        let (channel_id, message_id, author_id, content, edited_at) = row?;
        messages.push(ScyllaMessage { channel_id, message_id, author_id, content, edited_at });
    }
    Ok(messages)
}

pub async fn find_one(
    client: &ScyllaClient,
    channel_id: i64,
    message_id: i64,
) -> anyhow::Result<Option<ScyllaMessage>> {
    let result = client.session().query_unpaged(
        "SELECT channel_id, message_id, author_id, content, edited_at FROM pulsar.messages WHERE channel_id = ? AND message_id = ? LIMIT 1",
        (channel_id, message_id),
    ).await?;

    let rows_result = result.into_rows_result()?;
    let mut rows = rows_result.rows::<(i64, i64, i64, Vec<u8>, Option<i64>)>()?;
    match rows.next() {
        Some(Ok((channel_id, message_id, author_id, content, edited_at))) =>
            Ok(Some(ScyllaMessage { channel_id, message_id, author_id, content, edited_at })),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

pub async fn update_content(
    client: &ScyllaClient,
    channel_id: i64,
    message_id: i64,
    content: Vec<u8>,
    edited_at: i64,
) -> anyhow::Result<()> {
    client.session().query_unpaged(
        "UPDATE pulsar.messages SET content = ?, edited_at = ? WHERE channel_id = ? AND message_id = ?",
        (&content, edited_at, channel_id, message_id),
    ).await?;
    Ok(())
}

pub async fn delete_one(client: &ScyllaClient, channel_id: i64, message_id: i64) -> anyhow::Result<()> {
    client.session().query_unpaged(
        "DELETE FROM pulsar.messages WHERE channel_id = ? AND message_id = ?",
        (channel_id, message_id),
    ).await?;
    Ok(())
}

pub async fn delete_by_channel(client: &ScyllaClient, channel_id: i64) -> anyhow::Result<()> {
    client.session().query_unpaged(
        "DELETE FROM pulsar.messages WHERE channel_id = ?",
        (channel_id,),
    ).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scylla_message_struct_fields() {
        let msg = ScyllaMessage {
            channel_id: 1,
            message_id: 2,
            author_id: 3,
            content: b"ciphertext".to_vec(),
            edited_at: None,
        };
        assert_eq!(msg.channel_id, 1);
        assert_eq!(msg.content, b"ciphertext");
        assert!(msg.edited_at.is_none());
    }
}
