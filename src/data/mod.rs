use async_std::sync::Arc;

use mongodb::error::Error as MongoError;

use crate::file::FileProvider;
use crate::mongo::models::document::Document;
use crate::mongo::wrapper::MongoWrapper;

pub enum DataResult {
    Ok,
    Error(MongoError),
}

#[derive(Clone)]
pub struct Data {
    pub file: Arc<Box<dyn FileProvider>>,
    mongo: MongoWrapper,
}

impl Data {
    pub fn new(file: Box<dyn FileProvider>, mongo: MongoWrapper) -> Self {
        Data {
            file: Arc::new(file),
            mongo,
        }
    }

    pub async fn get_all_documents(&self) -> Option<Vec<Document>> {
        self.mongo.get_all::<Document, _>("date").await
    }

    pub async fn get_documents_by_token<S: AsRef<str>>(&self, value: S) -> Option<Vec<Document>> {
        self.mongo
            .get_all_by::<Document, _>("token", value.as_ref(), "date")
            .await
    }

    pub async fn create_document(&self, document: Document) -> DataResult {
        match self.mongo.create::<Document>(document).await {
            Ok(_) => DataResult::Ok,
            Err(e) => DataResult::Error(e),
        }
    }
}
