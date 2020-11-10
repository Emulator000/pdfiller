use chrono::{DateTime, Utc};

use simple_cache::CacheItem;

use mongodb::bson::Document as MongoDocument;

use crate::mongo::models::Model;

#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub token: String,
    pub file: String,
    pub date: DateTime<Utc>,
}

impl CacheItem for Document {}

impl Model for Document {
    fn name() -> &'static str {
        "document"
    }

    fn key(&self) -> String {
        format!(
            "{}_{}",
            Self::model_key::<Self, _>(Some(&self.token)),
            &self.file
        )
    }

    fn to_document(&self) -> MongoDocument {
        doc! {
            "token": self.token.clone(),
            "file": self.file.clone(),
            "date": self.date.clone(),
        }
    }
}
