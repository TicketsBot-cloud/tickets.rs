use common::event_forwarding::Event;
use deadpool_redis::{
    redis::{cmd, Value},
    Pool,
};
use tracing::warn;
use crate::Result;

const BLOCK_MS: usize = 1000;

pub struct Consumer {
    pool: Pool,
    stream: String,
    group: String,
    consumer_name: String,
}

impl Consumer {
    pub async fn new(
        pool: Pool,
        stream: String,
        group: String,
        consumer_name: String,
    ) -> Result<Self> {
        let mut conn = pool.get().await?;

        let result: std::result::Result<(), deadpool_redis::redis::RedisError> = cmd("XGROUP")
            .arg("CREATE")
            .arg(&stream)
            .arg(&group)
            .arg("$")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        if let Err(e) = result {
            if !e.to_string().contains("BUSYGROUP") {
                return Err(e.into());
            }
        }

        Ok(Self {
            pool,
            stream,
            group,
            consumer_name,
        })
    }

    /// Acks every entry it reads, including ones that fail to parse, so none stay pending.
    pub async fn recv_batch(&self, count: usize) -> Result<Vec<Event>> {
        loop {
            let mut conn = self.pool.get().await?;

            let result: Value = cmd("XREADGROUP")
                .arg("GROUP")
                .arg(&self.group)
                .arg(&self.consumer_name)
                .arg("BLOCK")
                .arg(BLOCK_MS)
                .arg("COUNT")
                .arg(count)
                .arg("STREAMS")
                .arg(&self.stream)
                .arg(">")
                .query_async(&mut conn)
                .await?;

            let entries = parse_stream_entries(&result);
            if entries.is_empty() {
                continue;
            }

            cmd("XACK")
                .arg(&self.stream)
                .arg(&self.group)
                .arg(
                    entries
                        .iter()
                        .map(|(id, _)| id.as_str())
                        .collect::<Vec<_>>(),
                )
                .query_async::<_, i64>(&mut conn)
                .await?;

            let events: Vec<Event> = entries
                .into_iter()
                .filter_map(|(_, payload)| payload)
                .filter_map(|payload| match serde_json::from_str::<Event>(&payload) {
                    Ok(ev) => Some(ev),
                    Err(e) => {
                        warn!(error = %e, "Failed to deserialise stream message, skipping");
                        None
                    }
                })
                .collect();

            if !events.is_empty() {
                return Ok(events);
            }
        }
    }
}

/// Parses an XREADGROUP reply into (message_id, data_field_value) pairs, in stream order.
///
/// XREADGROUP returns: [[stream_name, [[msg_id, [field, value, ...]], ...]]]
fn parse_stream_entries(value: &Value) -> Vec<(String, Option<String>)> {
    let Value::Bulk(streams) = value else {
        return Vec::new();
    };
    let Some(Value::Bulk(stream_parts)) = streams.first() else {
        return Vec::new();
    };
    let Some(Value::Bulk(messages)) = stream_parts.get(1) else {
        return Vec::new();
    };

    messages
        .iter()
        .filter_map(|message| {
            let Value::Bulk(message) = message else {
                return None;
            };
            let msg_id = as_string(message.first()?)?;
            let payload = match message.get(1) {
                Some(Value::Bulk(fields)) => fields.chunks(2).find_map(|pair| match pair {
                    [name, value] if as_string(name).as_deref() == Some("data") => as_string(value),
                    _ => None,
                }),
                _ => None,
            };
            Some((msg_id, payload))
        })
        .collect()
}

fn as_string(value: &Value) -> Option<String> {
    match value {
        Value::Data(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        Value::Status(s) => Some(s.clone()),
        _ => None,
    }
}
