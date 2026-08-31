use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_addr: String,
    pub worker_url: String,
    pub auth_header_name: String,
    pub auth_header_value: String,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_timeout_seconds() -> u64 {
    5
}

impl Config {
    pub fn from_envvar() -> Config {
        envy::from_env().unwrap()
    }
}
