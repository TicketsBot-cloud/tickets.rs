use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub cache_uri: String,
    pub redis_addr: String,
}

impl Config {
    pub fn new() -> Config {
        envy::from_env().expect("failed to parse config")
    }

    pub fn get_redis_uri(&self) -> String {
        format!("redis://{}/", self.redis_addr)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
