use async_trait::async_trait;
use common::event_forwarding;
use deadpool_redis::Pool;
use event_stream::Publisher;
use model::Snowflake;

use crate::{Config, Result};

use super::EventForwarder;

pub struct RedisStreamEventForwarder {
    publisher: Publisher,
}

impl RedisStreamEventForwarder {
    pub fn new(pool: Pool) -> Self {
        let publisher = Publisher::new(pool);
        Self { publisher }
    }
}

#[async_trait]
impl EventForwarder for RedisStreamEventForwarder {
    #[tracing::instrument(skip(self, _config, event))]
    async fn forward_event(
        &self,
        _config: &Config,
        event: event_forwarding::Event,
        _guild_id: Option<Snowflake>,
    ) -> Result<()> {
        self.publisher.send(&event).await?;
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}
