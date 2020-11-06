use async_std::sync::Arc;

use redis::RedisError;

use crate::file::FileProvider;
use crate::redis::models::document::Document;
use crate::redis::wrapper::RedisWrapper;

pub type FileType = Arc<Box<dyn FileProvider>>;

pub enum DataResult {
    Ok,
    Error(RedisError),
}

#[derive(Clone)]
pub struct Data {
    pub file: FileType,
    redis: RedisWrapper,
}

impl Data {
    pub fn new(file: Box<dyn FileProvider>, redis: RedisWrapper) -> Self {
        Data {
            file: Arc::new(file),
            redis,
        }
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
