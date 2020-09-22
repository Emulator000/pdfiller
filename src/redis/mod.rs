mod cache;
pub mod models;
pub mod wrapper;

use std::sync::Arc;

use redis::{AsyncCommands, RedisResult};

use crate::config::RedisConfig;
use crate::redis::cache::Cache;
use crate::redis::models::Model;

#[derive(Clone)]
pub struct Redis {
    client: Arc<redis::Client>,
    cache: Cache,
}

impl Redis {
    const DEFAULT_PORT: u32 = 6379;

    pub async fn new(redis_config: &RedisConfig) -> Self {
        Redis {
            client: Arc::new(
                match redis::Client::open(
                    format!(
                        "redis://{}:{}/",
                        redis_config.host,
                        redis_config.port.unwrap_or(Self::DEFAULT_PORT),
                    )
                    .as_str(),
                ) {
                    Ok(connection) => connection,
                    Err(e) => panic!("Error {:#?} connecting to {}", e, redis_config.host),
                },
            ),
            cache: Cache::new(),
        }
    }

    pub async fn insert<T: 'static + Model>(&self, model: T) -> RedisResult<()> {
        unimplemented!()
    }

    pub async fn insert_multiple<T: 'static + Model>(&self, model: T) -> RedisResult<()> {
        unimplemented!()
    }

    pub async fn update_one<T: 'static + Model, S: AsRef<str>>(
        &self,
        name: S,
        id: S,
        model: T,
    ) -> RedisResult<()> {
        unimplemented!()
    }

    // ToDo: find a way to introduce a cache here!
    pub async fn get<T: 'static + Model, S: AsRef<str>>(
        &self,
        filter: Option<S>,
    ) -> Option<Vec<T>> {
        unimplemented!()
    }

    pub async fn get_one<T: 'static + Model, S: AsRef<str>>(
        &self,
        name: S,
        id: S,
    ) -> Option<Arc<T>> {
        unimplemented!()
    }

    pub async fn delete_one<T: Model, S: AsRef<str>>(&self, name: S, id: S) -> RedisResult<()> {
        unimplemented!()
    }
}
