use super::routes;
use crate::{Config, Error};
use axum::routing::get;
use axum::{Extension, Router};
use cache::Cache;
use deadpool_redis::redis::AsyncCommands;
use deadpool_redis::Pool as RedisPool;
use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Serialize, Clone, Debug)]
pub struct ShardInfo {
    pub shard_id: u16,
    pub cluster_id: u16,
    pub guild_count: usize,
    pub latency_ms: u64,
    pub uptime_seconds: u64,
    pub status: String,
    pub last_seen: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct ClusterInfo {
    pub cluster_id: u16,
    pub shards: Vec<ShardInfo>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ShardsSnapshot {
    pub success: bool,
    pub total_shards: u16,
    pub total_clusters: u16,
    pub connected_shards: u16,
    pub total_guilds: usize,
    pub clusters: Vec<ClusterInfo>,
}

impl Default for ShardsSnapshot {
    fn default() -> Self {
        Self {
            success: true,
            total_shards: 0,
            total_clusters: 0,
            connected_shards: 0,
            total_guilds: 0,
            clusters: Vec::new(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct ShardStatusBlob {
    shard_id: u16,
    cluster_id: u16,
    #[serde(default)]
    cluster_size: u16,
    num_shards: u16,
    guild_count: usize,
    latency_ms: u64,
    connected: bool,
    uptime_seconds: u64,
    last_seen: u64,
}

const STALE_THRESHOLD_SECS: u64 = 45;

pub struct Server<T: Cache> {
    pub config: Config,
    pub cache: T,
    pub count: AtomicUsize,
    pub redis: RedisPool,
    pub shards_snapshot: RwLock<ShardsSnapshot>,
}

impl<T: Cache> Server<T> {
    pub fn new(config: Config, cache: T, redis: RedisPool) -> Server<T> {
        Server {
            config,
            cache,
            count: AtomicUsize::new(0),
            redis,
            shards_snapshot: RwLock::new(ShardsSnapshot::default()),
        }
    }

    pub async fn start(self) -> Result<(), Error> {
        let server = Arc::new(self);

        server.clone().start_update_loop();
        server.clone().start_shard_status_loop();

        let app = Router::new()
            .route("/total", get(routes::total_handler::<T>))
            .route("/total/prometheus", get(routes::prometheus_handler::<T>))
            .route("/shards", get(routes::shards_handler::<T>))
            .layer(Extension(server.clone()));

        let addr = &server.config.server_addr[..].parse()?;

        hyper::Server::bind(addr)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }

    fn start_update_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                let count = match self.cache.get_guild_count().await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Error while getting guild count: {}", e);
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                self.count.store(count, Ordering::Relaxed);

                sleep(Duration::from_secs(15)).await;
            }
        });
    }

    fn start_shard_status_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                if let Err(e) = self.refresh_shard_status().await {
                    warn!("Failed to refresh shard status: {}", e);
                }
                sleep(Duration::from_secs(10)).await;
            }
        });
    }

    async fn refresh_shard_status(&self) -> Result<(), Error> {
        let keys = self.scan_shard_status_keys().await?;

        if keys.is_empty() {
            let snapshot = ShardsSnapshot::default();
            if let Ok(mut guard) = self.shards_snapshot.write() {
                *guard = snapshot;
            }
            return Ok(());
        }

        let mut conn = self.redis.get().await?;
        let values: Vec<Option<String>> = conn.mget(&keys).await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut blobs: Vec<ShardStatusBlob> = Vec::new();
        for (i, val) in values.into_iter().enumerate() {
            let Some(json_str) = val else { continue };
            match serde_json::from_str::<ShardStatusBlob>(&json_str) {
                Ok(blob) => blobs.push(blob),
                Err(e) => {
                    let key = keys.get(i).map(|k| k.as_str()).unwrap_or("unknown");
                    warn!("Failed to parse shard status blob for key {}: {}", key, e);
                }
            }
        }

        let total: u16 = blobs.iter().map(|b| b.num_shards).max().unwrap_or(0);
        let cluster_size: u16 = blobs
            .iter()
            .map(|b| b.cluster_size)
            .filter(|&cs| cs > 0)
            .max()
            .unwrap_or(0);

        if total == 0 {
            let snapshot = ShardsSnapshot::default();
            if let Ok(mut guard) = self.shards_snapshot.write() {
                *guard = snapshot;
            }
            return Ok(());
        }

        let blob_map: BTreeMap<u16, &ShardStatusBlob> =
            blobs.iter().map(|b| (b.shard_id, b)).collect();

        let mut clusters_map: BTreeMap<u16, Vec<ShardInfo>> = BTreeMap::new();
        let mut connected_count: u16 = 0;
        let mut total_guilds: usize = 0;

        for shard_id in 0..total {
            let (info, is_connected) = match blob_map.get(&shard_id) {
                Some(blob) => {
                    let stale = now.saturating_sub(blob.last_seen) > STALE_THRESHOLD_SECS;
                    let connected = blob.connected && !stale;
                    let status = if connected {
                        "connected".to_string()
                    } else {
                        "disconnected".to_string()
                    };

                    (
                        ShardInfo {
                            shard_id,
                            cluster_id: blob.cluster_id,
                            guild_count: blob.guild_count,
                            latency_ms: blob.latency_ms,
                            uptime_seconds: blob.uptime_seconds,
                            status,
                            last_seen: blob.last_seen,
                        },
                        connected,
                    )
                }
                None => {
                    let cid = shard_id.checked_div(cluster_size).unwrap_or(0);
                    (
                        ShardInfo {
                            shard_id,
                            cluster_id: cid,
                            guild_count: 0,
                            latency_ms: 0,
                            uptime_seconds: 0,
                            status: "disconnected".to_string(),
                            last_seen: 0,
                        },
                        false,
                    )
                }
            };

            if is_connected {
                connected_count += 1;
                total_guilds += info.guild_count;
            }

            clusters_map.entry(info.cluster_id).or_default().push(info);
        }

        let total_clusters = clusters_map.len() as u16;

        let clusters: Vec<ClusterInfo> = clusters_map
            .into_iter()
            .map(|(cluster_id, mut shards)| {
                shards.sort_by_key(|s| s.shard_id);
                ClusterInfo { cluster_id, shards }
            })
            .collect();

        let snapshot = ShardsSnapshot {
            success: true,
            total_shards: total,
            total_clusters,
            connected_shards: connected_count,
            total_guilds,
            clusters,
        };

        if let Ok(mut guard) = self.shards_snapshot.write() {
            *guard = snapshot;
        }

        Ok(())
    }

    async fn scan_shard_status_keys(&self) -> Result<Vec<String>, Error> {
        let mut conn = self.redis.get().await?;
        let mut keys = Vec::new();
        let mut cursor: u64 = 0;
        loop {
            let (next_cursor, batch): (u64, Vec<String>) = deadpool_redis::redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("tickets:shard_status:public:*")
                .arg("COUNT")
                .arg(1000)
                .query_async(&mut conn)
                .await?;
            keys.extend(batch);
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        Ok(keys)
    }
}
