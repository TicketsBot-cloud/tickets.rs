use common::event_forwarding;
use deadpool_redis::{redis::cmd, Pool};
use crate::Result;

const STREAM_KEY: &str = "stream:gateway-events";

pub struct Publisher {
    pool: Pool,
    max_len: usize,
}

impl Publisher {
    pub fn new(pool: Pool, max_len: usize) -> Self {
        Self { pool, max_len }
    }

    pub async fn send(&self, ev: &event_forwarding::Event) -> Result<()> {
        let payload = serde_json::to_string(ev)?;
        let mut conn = self.pool.get().await?;

        cmd("XADD")
            .arg(STREAM_KEY)
            .arg("MAXLEN")
            .arg("~")
            .arg(self.max_len)
            .arg("*")
            .arg("data")
            .arg(&payload)
            .query_async::<_, String>(&mut conn)
            .await?;

        Ok(())
    }
}
