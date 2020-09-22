use async_std::sync::Arc;

use redis::RedisResult;

use crate::redis::models::Model;
use crate::redis::Redis;

#[derive(Clone)]
pub struct RedisWrapper {
    redis: Redis,
}

impl RedisWrapper {
    pub fn new(redis: Redis) -> Self {
        Self { redis }
    }

    /// Generic
    pub async fn get_all<T: 'static + Model>(&self) -> Option<Vec<T>> {
        self.redis.get::<T, &str>(None).await
    }

    pub async fn get_all_by<T: 'static + Model, S: AsRef<str>>(&self, value: S) -> Option<Vec<T>> {
        self.redis.get::<T, _>(Some(value)).await
    }

    pub async fn get<T: 'static + Model, S: AsRef<str>>(&self, value: S) -> Option<Arc<T>> {
        self.redis.get_one::<T, _>(value).await
    }

    pub async fn create<T: 'static + Model>(&self, model: T) -> RedisResult<()> {
        self.redis.insert::<T>(model).await
    }

    pub async fn update<T: 'static + Model, S: AsRef<str>>(
        &self,
        value: S,
        model: T,
    ) -> RedisResult<()> {
        self.redis.update_one::<T, _>(value, model).await
    }

    pub async fn delete<T: Model, S: AsRef<str>>(&self, value: S) -> RedisResult<()> {
        self.redis.delete_one::<T, _>(value).await
    }
}
