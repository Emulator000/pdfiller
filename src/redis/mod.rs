pub mod models;
pub mod wrapper;

use std::sync::Arc;

use async_std::sync::{RwLock, RwLockWriteGuard};

use redis::RedisResult;

use simple_cache::Cache;

use crate::config::MongoConfig;
use crate::logger::Logger;
use crate::redis::models::Model;

#[derive(Clone)]
pub struct Redis {
    client: Arc<redis::Client>,
    connection: Arc<RwLock<redis::aio::Connection>>,
    cache: Cache<String>,
}

impl Redis {
    const DEFAULT_PORT: u32 = 27017;

    pub async fn new(redis_config: &MongoConfig) -> Self {
        let client = redis::Client::open(format!(
            "redis://{}:{}",
            redis_config.host,
            redis_config.port.unwrap_or(Self::DEFAULT_PORT)
        ))
        .unwrap_or_else(|e| panic!("Error {:#?} connecting to {}", e, redis_config.host));

        Redis {
            connection: Arc::new(RwLock::new(
                Self::get_connection(&client)
                    .await
                    .unwrap_or_else(|e| panic!("Error {:#?} can't get a Redis connection.", e)),
            )),
            client: Arc::new(client),
            cache: Cache::new(),
        }
    }

    async fn connection(&self) -> RwLockWriteGuard<'_, redis::aio::Connection> {
        let connected = redis::cmd("PING")
            .query_async::<_, String>(&mut *self.connection.write().await)
            .await
            .is_ok();

        if connected {
            self.connection.write().await
        } else {
            Logger::log("Recovering a lost Redis connection.");

            match Self::get_connection(&self.client).await {
                Ok(connection) => {
                    *self.connection.write().await = connection;

                    self.connection.write().await
                }
                Err(e) => {
                    sentry::capture_error(&e);

                    panic!("Error {:#?} can't get a Redis connection.", e);
                }
            }
        }
    }

    async fn get_connection(client: &redis::Client) -> RedisResult<redis::aio::Connection> {
        client.get_async_connection().await
    }

    pub async fn insert<T: Model>(&self, model: T) -> RedisResult<()> {
        redis::cmd("SET")
            .arg(&[
                model.key(),
                serde_json::to_string(&model).unwrap_or(String::new()),
            ])
            .query_async(&mut *self.connection().await)
            .await
    }

    pub async fn update_one<T: 'static + Model>(&self, model: T) -> RedisResult<()> {
        let key = model.key();
        let result = redis::cmd("SET")
            .arg(&[
                &key,
                &serde_json::to_string(&model).unwrap_or(String::new()),
            ])
            .query_async(&mut *self.connection().await)
            .await;

        if result.is_ok() {
            self.cache.insert::<T>(key, Some(model)).await;
        }

        result
    }

    // ToDo: find a way to introduce a cache here!
    pub async fn get<T: Model, S: AsRef<str>>(&self, value: Option<S>) -> Option<Vec<T>> {
        let key = if let Some(value) = value {
            format!("{}_*", T::model_key::<T, S>(Some(value)))
        } else {
            T::model_key::<T, S>(None)
        };

        let connection = &mut *self.connection().await;
        match redis::cmd("KEYS")
            .arg(&key)
            .query_async::<_, Vec<String>>(connection)
            .await
        {
            Ok(res) => {
                let mut results = Vec::new();
                for key in res {
                    match redis::cmd("GET")
                        .arg(&key)
                        .query_async::<_, String>(connection)
                        .await
                    {
                        Ok(res) => match serde_json::from_str::<T>(&res) {
                            Ok(res) => {
                                results.push(res);
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
                        }
                    }
                }

                Some(results)
            }
            Err(e) => {
                sentry::capture_error(&e);

                Logger::log(format!(
                    "Error getting keys with pattern {}: {:#?}",
                    &key, e
                ));

                None
            }
        }
    }

    #[allow(dead_code)]
    pub async fn get_one<T: 'static + Model, S: AsRef<str>>(&self, value: S) -> Option<Arc<T>> {
        let key = T::model_key::<T, S>(Some(value));
        if let Some(value) = self.cache.get::<T, _>(&key).await {
            value
        } else {
            match redis::cmd("GET")
                .arg(&key)
                .query_async::<_, String>(&mut *self.connection().await)
                .await
            {
                Ok(res) => match serde_json::from_str::<T>(&res) {
                    Ok(res) => {
                        self.cache.insert::<T>(key.clone(), Some(res)).await;
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

                    self.cache.insert::<T>(key.clone(), None).await;
                }
            };

            self.cache.get::<T, _>(&key).await.unwrap()
        }
    }

    #[allow(dead_code)]
    pub async fn delete_one<T: Model>(&self, model: T) -> RedisResult<()> {
        let key = model.key();
        let result = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut *self.connection().await)
            .await;

        match &result {
            Ok(_) => {
                self.cache.remove(&key).await;
            }
            Err(e) => {
                sentry::capture_error(e);

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
