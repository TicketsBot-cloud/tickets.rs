use std::sync::Arc;

use sharder::{
    await_shutdown, build_redis, load_keyring_from_env, Config, RedisSessionStore, ShardManager,
    WhitelabelShardManager,
};

#[cfg(feature = "use-sentry")]
use sharder::setup_sentry;

use database::{sqlx::postgres::PgPoolOptions, Database};

use sharder::event_forwarding::RedisStreamEventForwarder;
use tracing::info;

#[cfg(feature = "use-jemalloc")]
use jemallocator::Jemalloc;
#[cfg(feature = "use-mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "use-jemalloc")]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "use-mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// #[cfg(not(feature = "whitelabel"))]
// fn main() {
//     panic!("Started whitelabel sharder without whitelabel feature flag")
// }

// #[cfg(feature = "whitelabel")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_envvar();

    // Refuse to run without a usable keyring rather than start shards that
    // cannot decrypt any token.
    let keyring = Arc::new(load_keyring_from_env()?);

    #[cfg(feature = "use-sentry")]
    let _guard = setup_sentry(&config);

    #[cfg(not(feature = "use-sentry"))]
    env_logger::init();

    // init db
    let db_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(config.database_threads);
    let database = Arc::new(Database::connect(&config.database_uri[..], db_opts).await?);

    // init redis
    let redis = Arc::new(build_redis(&config));

    let session_store = RedisSessionStore::new(
        Arc::clone(&redis),
        "tickets:resume:whitelabel".to_string(),
        300,
    );

    let event_forwarder = Arc::new(RedisStreamEventForwarder::new(
        (*redis).clone(),
        config.gateway_stream_max_len,
    ));

    let sm = Arc::new(WhitelabelShardManager::new(
        config,
        database,
        redis,
        session_store,
        event_forwarder,
        keyring,
    ));

    Arc::clone(&sm).connect().await;

    Arc::clone(&sm).listen_status_updates().await.unwrap();
    Arc::clone(&sm).listen_new_tokens().await.unwrap();
    Arc::clone(&sm).listen_delete().await.unwrap();

    await_shutdown()
        .await
        .expect("Failed to wait for shutdown signal");
    info!("Received shutdown signal");

    sm.shutdown().await;
    info!("Shard manager shutdown gracefully");

    Ok(())
}
