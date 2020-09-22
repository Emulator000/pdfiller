mod cache;
pub mod models;
pub mod wrapper;

use std::sync::Arc;

use async_std::sync::RwLock;

use redis::RedisResult;

use crate::config::RedisConfig;
use crate::logger::Logger;
use crate::redis::cache::Cache;
use crate::redis::models::Model;

#[derive(Clone)]
pub struct Redis {
    connection: Arc<RwLock<redis::aio::Connection>>,
    cache: Cache,
}

impl Redis {
    const DEFAULT_PORT: u32 = 6379;

    pub async fn new(redis_config: &RedisConfig) -> Self {
        Redis {
            connection: Arc::new(RwLock::new(
                redis::Client::open(format!(
                    "redis://{}:{}",
                    redis_config.host,
                    redis_config.port.unwrap_or(Self::DEFAULT_PORT)
                ))
                .unwrap_or_else(|e| panic!("Error {:#?} connecting to {}", e, redis_config.host))
                .get_async_connection()
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "Error {:#?} can't get a Redis connection to {}",
                        e, redis_config.host
                    )
                }),
            )),
            cache: Cache::new(),
        }
    }

    pub async fn insert<T: Model>(&self, model: T) -> RedisResult<()> {
        redis::cmd("SET")
            .arg(&[
                model.key(),
                serde_json::to_string(&model).unwrap_or(String::new()),
            ])
            .query_async(&mut *self.connection.write().await)
            .await
    }

    pub async fn update_one<T: 'static + Model, S: AsRef<str>>(
        &self,
        value: S,
        model: T,
    ) -> RedisResult<()> {
        let key = T::model_key::<T, S>(Some(value));
        let result = redis::cmd("SET")
            .arg(&[
                &key,
                &serde_json::to_string(&model).unwrap_or(String::new()),
            ])
            .query_async(&mut *self.connection.write().await)
            .await;
        if result.is_ok() {
            self.cache.set::<T, _>(&key, Some(model)).await;
        }

        result
    }

    // ToDo: find a way to introduce a cache here!
    pub async fn get<T: Model, S: AsRef<str>>(&self, value: Option<S>) -> Option<Vec<T>> {
        match redis::cmd("MGET")
            .arg(&T::model_key::<T, S>(value))
            .query_async::<_, Vec<_>>(&mut *self.connection.write().await)
            .await
        {
            Ok(res) => {
                let deserialized: Vec<_> = res
                    .iter()
                    .map(|value: &String| serde_json::from_str::<T>(&value))
                    .collect();

                let mut results = Vec::new();
                for deserialize in deserialized {
                    match deserialize {
                        Ok(res) => {
                            results.push(res);
                        }
                        _ => {}
                    }
                }

                Some(results)
            }
            Err(_) => None,
        }
    }

    pub async fn get_one<T: 'static + Model, S: AsRef<str>>(&self, value: S) -> Option<Arc<T>> {
        let key = T::model_key::<T, S>(Some(value));
        if let Some(value) = self.cache.get::<T, _>(&key).await {
            value
        } else {
            match redis::cmd("GET")
                .arg(&key)
                .query_async::<_, String>(&mut *self.connection.write().await)
                .await
            {
                Ok(res) => match serde_json::from_str::<T>(&res) {
                    Ok(res) => {
                        self.cache.set::<T, _>(&key, Some(res)).await;
                    }
                    _ => {}
                },
                Err(e) => {
                    sentry::capture_error(&e);

                    Logger::log(format!(
                        "Error getting {} with key {}: {:#?}",
                        T::name(),
                        &key,
                        e
                    ));

                    self.cache.set::<T, _>(&key, None).await;
                }
            };

            self.cache.get::<T, _>(&key).await.unwrap()
        }
    }

    pub async fn delete_one<T: Model, S: AsRef<str>>(&self, value: S) -> RedisResult<()> {
        let key = T::model_key::<T, S>(Some(value));
        let result = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut *self.connection.write().await)
            .await;

        match &result {
            Ok(_) => {
                self.cache.del(&key).await;
            }
            Err(e) => {
                Logger::log(format!(
                    "Error deleting {} with key {}: {:#?}",
                    T::name(),
                    &key,
                    e
                ));
            }
        };

        result
    }
}
