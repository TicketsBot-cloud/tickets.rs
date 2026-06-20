use common::event_forwarding;
use deadpool_redis::{redis::cmd, Pool};
use crate::Result;

const STREAM_KEY: &str = "stream:gateway-events";
const MAX_LEN: usize = 50_000;

pub struct Publisher {
    pool: Pool,
}

impl Publisher {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn send(&self, ev: &event_forwarding::Event) -> Result<()> {
        let payload = serde_json::to_string(ev)?;
        let mut conn = self.pool.get().await?;

        cmd("XADD")
            .arg(STREAM_KEY)
            .arg("MAXLEN")
            .arg("~")
            .arg(MAX_LEN)
            .arg("*")
            .arg("data")
            .arg(&payload)
            .query_async::<_, String>(&mut conn)
            .await?;

        Ok(())
    }
}
