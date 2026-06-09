use std::sync::Arc;

use cache::Cache;
use deadpool::managed::PoolConfig;
use deadpool::Runtime;
use deadpool_redis::{Config as RedisConfig, Pool};
use event_stream::Consumer;
use tracing::debug;

use crate::{Config, Result};

use super::worker::Worker;

pub struct Manager<C: Cache> {
    config: Config,
    cache: Arc<C>,
}

impl<C: Cache> Manager<C> {
    pub fn new(config: Config, cache: C) -> Self {
        let cache = Arc::new(cache);

        Self { config, cache }
    }

    pub async fn start(&self) -> Result<()> {
        let stream = self.config.redis_stream.clone();
        let group = self.config.consumer_group.clone();

        debug!(%stream, %group, "Building Redis pool for consumer");
        let pool = build_pool(&self.config);

        for i in 0..self.config.workers {
            let consumer_name = format!("worker-{}", i);

            debug!(%stream, %group, %consumer_name, "Creating consumer");
            let consumer = Arc::new(
                Consumer::new(
                    pool.clone(),
                    stream.clone(),
                    group.clone(),
                    consumer_name,
                )
                .await?,
            );

            let worker = Worker::new(i, consumer, Arc::clone(&self.cache));

            tokio::spawn(async move {
                worker.run().await;
            });
        }

        Ok(())
    }
}

fn build_pool(config: &Config) -> Pool {
    let mut cfg = RedisConfig::from_url(config.get_redis_uri());
    cfg.pool = Some(PoolConfig::new(config.workers));

    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool for consumer")
}
