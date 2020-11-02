use redis::RedisError;

use crate::redis::models::document::Document;
use crate::redis::wrapper::RedisWrapper;

pub enum DataResult {
    Ok,
    Error(RedisError),
}

#[derive(Clone)]
pub struct Data {
    redis: RedisWrapper,
}

impl Data {
    pub fn new(redis: RedisWrapper) -> Self {
        Data { redis }
    }

    pub async fn get_all_documents(&self) -> Option<Vec<Document>> {
        self.redis.get_all::<Document>().await
    }

    pub async fn get_documents_by_token<S: AsRef<str>>(&self, value: S) -> Option<Vec<Document>> {
        self.redis.get_all_by::<Document, _>(value).await
    }

    pub async fn create_document(&self, document: Document) -> DataResult {
        match self.redis.create::<Document>(document).await {
            Ok(_) => DataResult::Ok,
            Err(e) => DataResult::Error(e),
        }
    }
}
