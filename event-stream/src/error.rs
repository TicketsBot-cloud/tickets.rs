pub type Result<T> = std::result::Result<T, StreamError>;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Redis error: {0}")]
    RedisError(#[from] deadpool_redis::redis::RedisError),

    #[error("Redis pool error: {0}")]
    PoolError(#[from] deadpool_redis::PoolError),

    #[error("serde_json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("no payload in stream message")]
    NoPayload,
}

impl<T> From<StreamError> for Result<T> {
    fn from(e: StreamError) -> Self {
        Err(e)
    }
}
