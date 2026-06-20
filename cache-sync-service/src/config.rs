use crate::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub workers: usize,
    pub redis_addr: String,
    pub redis_password: Option<String>,
    pub redis_stream: String,
    pub consumer_group: String,
    pub postgres_uri: String,
    pub metric_server_addr: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        envy::from_env().map_err(Into::into)
    }

    pub fn get_redis_uri(&self) -> String {
        match &self.redis_password {
            Some(pwd) => format!("redis://:{}@{}/", pwd, self.redis_addr),
            None => format!("redis://{}/", self.redis_addr),
        }
    }
}
