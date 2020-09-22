use std::sync::Arc;

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

    pub async fn get_all_by<T: 'static + Model, S: AsRef<str>>(
        &self,
        key: S,
        value: S,
    ) -> Option<Vec<T>> {
        self.redis
            .get::<T, &str>(Some(&format!(
                "WHERE {} = {}",
                key.as_ref(),
                value.as_ref()
            )))
            .await
    }

    pub async fn get<T: 'static + Model>(&self, id: i64) -> Option<Arc<T>> {
        self.redis.get_one::<T, _>("id", &id.to_string()).await
    }

    pub async fn get_by<T: 'static + Model, S: AsRef<str>>(
        &self,
        key: S,
        value: S,
    ) -> Option<Arc<T>> {
        self.redis.get_one::<T, _>(key, value).await
    }

    pub async fn create<T: 'static + Model>(&self, model: T) -> RedisResult<()> {
        self.redis.insert::<T>(model).await
    }

    pub async fn update<T: 'static + Model>(&self, model: T) -> RedisResult<()> {
        self.redis
            .update_one::<T, _>("id", &model.id().to_string(), model)
            .await
    }

    pub async fn delete<T: Model>(&self, id: i64) -> RedisResult<()> {
        self.redis.delete_one::<T, _>("id", &id.to_string()).await
    }
}
