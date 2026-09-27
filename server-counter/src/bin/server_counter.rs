use cache::PostgresCache;
use deadpool::managed::PoolConfig;
use deadpool::Runtime;
use deadpool_redis::Config as RedisConfig;
use log::info;
use server_counter::{http::Server, Config, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let config = Config::new();

    let cache = PostgresCache::connect(config.cache_uri.clone(), cache::Options::default(), 1)
        .await
        .map_err(Error::CacheError)?;

    let mut redis_cfg = RedisConfig::from_url(config.get_redis_uri());
    redis_cfg.pool = Some(PoolConfig::new(2));
    let redis = redis_cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool");

    let server = Server::new(config, cache, redis);
    info!("Starting server...");
    server.start().await
}
