use redis::RedisError;

use crate::config::ServiceConfig;
use crate::redis::models::document::Document;
use crate::redis::wrapper::RedisWrapper;

pub enum DataResult {
    Ok,
    Error(RedisError),
}

#[derive(Clone)]
pub struct Data {
    pub config: ServiceConfig,
    redis: RedisWrapper,
}

impl Data {
    pub fn new(config: ServiceConfig, redis: RedisWrapper) -> Self {
        Data { config, redis }
    }

    pub async fn get_all_documents(&self) -> Option<Vec<Document>> {
        self.redis
            .get_all::<Document>()
            .await
            .map(|documents| Self::sort_documents(documents))
    }

    pub async fn get_documents_by_token<S: AsRef<str>>(&self, value: S) -> Option<Vec<Document>> {
        self.redis
            .get_all_by::<Document, _>(value)
            .await
            .map(|documents| Self::sort_documents(documents))
    }

    pub async fn create_document(&self, document: Document) -> DataResult {
        match self.redis.create::<Document>(document).await {
            Ok(_) => DataResult::Ok,
            Err(e) => DataResult::Error(e),
        }
    }

    fn sort_documents(mut documents: Vec<Document>) -> Vec<Document> {
        documents.sort_by(|a, b| a.date.partial_cmp(&b.date).unwrap());

        documents
    }
}
