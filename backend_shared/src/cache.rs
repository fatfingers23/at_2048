use log::error;
use redis::AsyncCommands;
use redis::aio::{ConnectionManager, MultiplexedConnection};
use redis::{Connection, RedisResult};
use serde::{Deserialize, Serialize};

use redis::Commands;

struct Cache {
    redis_connection: ConnectionManager,
}

pub enum RedisFetchErrors {
    FromDbError,
    ParseError,
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
    ) -> RedisResult<String> {
        self.redis_connection
            .set_ex(redis_key, serde_json::to_string(&data).unwrap(), seconds)
            .await
    }

    pub async fn fetch_redis_json_object<T: for<'a> Deserialize<'a>>(
        redis_connection: &mut Connection,
        redis_key: &str,
    ) -> Result<T, RedisFetchErrors> {
        let val = redis::cmd("GET")
            .arg(redis_key)
            .query::<String>(redis_connection)
            .map_err(|err| {
                error!("Error fetching from redis: {}", err);
                RedisFetchErrors::FromDbError
            })?;

        let val: T = serde_json::from_str(&val).map_err(|err| {
            error!("Error parsing redis data: {}", err);
            RedisFetchErrors::ParseError
        })?;

        Ok(val)
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
        redis_connection: &mut Connection,
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
        match fetch_redis_json_object::<T>(redis_connection, redis_key).await {
            Ok(val) => Ok(val),
            Err(RedisFetchErrors::FromDbError) => {
                // If not in cache, execute the fallback function
                let result = fallback_fn().await?;

                // Write the result to cache
                write_to_cache_with_seconds(redis_connection, redis_key, &result, seconds).await;

                Ok(result)
            }
            Err(err) => Err(err),
        }
    }
}
