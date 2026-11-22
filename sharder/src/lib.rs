mod gateway;
pub use gateway::*;

mod manager;
pub use manager::{Options, ShardCount, ShardManager};

#[cfg(not(feature = "whitelabel"))]
pub use manager::PublicShardManager;

#[cfg(feature = "whitelabel")]
pub use manager::WhitelabelShardManager;

mod builders;
pub use builders::{build_redis, setup_sentry};

mod config;
pub use config::Config;

#[cfg(feature = "whitelabel")]
mod token_keyring;
#[cfg(feature = "whitelabel")]
pub use token_keyring::{load_keyring, load_keyring_from_env, KeyringLoadError};

#[cfg(feature = "metrics")]
pub mod metrics_server;

mod util;
pub use util::*;
