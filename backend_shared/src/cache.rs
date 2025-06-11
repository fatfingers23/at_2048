use redis::aio::ConnectionManager;
use redis::{AsyncCommands, RedisError};
use redis::{Connection, RedisResult};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DID_DOC_KEY_PREFIX: &str = "did_doc:";

pub struct Cache {
    redis_connection: ConnectionManager,
}

#[derive(Debug, Error)]
pub enum RedisFetchErrors {
    FromDbError,
    ParseError,
    Other(String),
}

impl std::fmt::Display for RedisFetchErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisFetchErrors::FromDbError => write!(f, "Error fetching from Redis database"),
            RedisFetchErrors::ParseError => write!(f, "Error parsing Redis data"),
            RedisFetchErrors::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl Cache {
    pub async fn new(connection: &str) -> RedisResult<Self> {
        let client = redis::Client::open(connection)?;
        let manager = client.get_connection_manager().await?;
        Ok(Self {
            redis_connection: manager,
        })
    }
}

impl Cache {
    pub async fn write_to_cache<T: Serialize>(
        &mut self,
        redis_key: String,
        data: T,
    ) -> redis::RedisResult<String> {
        self.redis_connection
            .set_ex(
                redis_key.clone(),
                serde_json::to_string(&data).unwrap(),
                3600,
            )
            .await
    }

    pub async fn write_to_cache_with_seconds<T: Serialize>(
        &mut self,
        redis_key: &str,
        data: T,
        seconds: u64,
    ) -> RedisResult<()> {
        self.redis_connection
            .set_ex(redis_key, serde_json::to_string(&data).unwrap(), seconds)
            .await
    }

    pub async fn fetch_redis_json_object<T: for<'a> Deserialize<'a>>(
        &mut self,
        redis_key: &str,
    ) -> Result<Option<T>, RedisFetchErrors> {
        let val: RedisResult<Option<String>> = self.redis_connection.get(redis_key).await;

        match val {
            Ok(val) => match val {
                None => Ok(None),
                Some(val) => Ok(serde_json::from_str(&val).map_err(|err| {
                    log::error!("Error parsing redis data: {}", err);
                    RedisFetchErrors::ParseError
                }))?,
            },
            Err(err) => Err(RedisFetchErrors::FromDbError),
        }
    }

    pub async fn fetch_redis<T: redis::FromRedisValue>(
        redis_connection: &mut Connection,
        redis_key: &str,
    ) -> Result<T, RedisFetchErrors> {
        let val = redis::cmd("GET")
            .arg(redis_key)
            .query::<T>(redis_connection)
            .map_err(|_| RedisFetchErrors::FromDbError)?;

        Ok(val)
    }

    pub async fn get_or_set<T, F, Fut>(
        &mut self,
        redis_key: &str,
        seconds: u64,
        fallback_fn: F,
    ) -> Result<T, RedisFetchErrors>
    where
        T: for<'a> Deserialize<'a> + Serialize,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, RedisFetchErrors>>,
    {
        // Try to get from cache first
        match self.fetch_redis_json_object::<T>(redis_key).await {
            Ok(Some(val)) => Ok(val),
            Ok(None) => {
                // If not in cache or error, execute the fallback function
                let result = fallback_fn().await?;

                // Write the result to cache
                self.write_to_cache_with_seconds(redis_key, &result, seconds)
                    .await
                    .map_err(|err| {
                        log::error!("Error fetching from redis: {}", err);
                        RedisFetchErrors::FromDbError
                    })?;

                Ok(result)
            }
            Err(err) => Err(err),
        }
    }
}
