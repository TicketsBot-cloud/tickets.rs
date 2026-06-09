use common::event_forwarding::Event;
use deadpool_redis::{redis::cmd, Pool};
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

    pub async fn recv(&self) -> Result<Event> {
        loop {
            let mut conn = self.pool.get().await?;

            let result: deadpool_redis::redis::Value = cmd("XREADGROUP")
                .arg("GROUP")
                .arg(&self.group)
                .arg(&self.consumer_name)
                .arg("BLOCK")
                .arg(BLOCK_MS)
                .arg("COUNT")
                .arg(1)
                .arg("STREAMS")
                .arg(&self.stream)
                .arg(">")
                .query_async(&mut conn)
                .await?;

            if let Some((msg_id, payload)) = parse_stream_response(&result) {
                cmd("XACK")
                    .arg(&self.stream)
                    .arg(&self.group)
                    .arg(&msg_id)
                    .query_async::<_, i64>(&mut conn)
                    .await?;

                match serde_json::from_str::<Event>(&payload) {
                    Ok(ev) => return Ok(ev),
                    Err(e) => {
                        warn!(error = %e, "Failed to deserialise stream message, skipping");
                        continue;
                    }
                }
            }
        }
    }
}

/// Parses the nested Redis stream response into (message_id, data_field_value).
///
/// XREADGROUP returns: [[stream_name, [[msg_id, [field, value, ...]]]]]
fn parse_stream_response(value: &deadpool_redis::redis::Value) -> Option<(String, String)> {
    use deadpool_redis::redis::Value;

    let streams = match value {
        Value::Bulk(v) => v,
        Value::Nil => return None,
        _ => return None,
    };

    let stream_entry = streams.first()?;
    let stream_parts = match stream_entry {
        Value::Bulk(v) => v,
        _ => return None,
    };

    let messages = match stream_parts.get(1)? {
        Value::Bulk(v) => v,
        _ => return None,
    };

    let message = match messages.first()? {
        Value::Bulk(v) => v,
        _ => return None,
    };

    let msg_id = match message.first()? {
        Value::Data(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        Value::Status(s) => s.clone(),
        _ => return None,
    };

    let fields = match message.get(1)? {
        Value::Bulk(v) => v,
        _ => return None,
    };

    let mut iter = fields.iter();
    while let Some(field) = iter.next() {
        let field_name = match field {
            Value::Data(bytes) => String::from_utf8_lossy(bytes),
            Value::Status(s) => std::borrow::Cow::Borrowed(s.as_str()),
            _ => continue,
        };

        if let Some(val) = iter.next() {
            if field_name == "data" {
                let payload = match val {
                    Value::Data(bytes) => String::from_utf8_lossy(bytes).into_owned(),
                    Value::Status(s) => s.clone(),
                    _ => return None,
                };
                return Some((msg_id, payload));
            }
        }
    }

    None
}
