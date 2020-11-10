use chrono::{DateTime, Utc};

use simple_cache::CacheItem;

use mongodb::bson::Document as MongoDocument;

use crate::mongo::models::Model;

#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<bson::oid::ObjectId>,
    pub token: String,
    pub file: String,
    pub date: DateTime<Utc>,
}

impl Document {
    pub fn default() -> Self {
        Self {
            id: None,
            token: "".into(),
            file: "".into(),
            date: Utc::now(),
        }
    }

    pub fn new(token: String, file: String) -> Self {
        Self {
            id: None,
            token,
            file,
            date: Utc::now(),
        }
    }
}

impl CacheItem for Document {}

impl Model for Document {
    fn name() -> &'static str {
        "document"
    }

    fn key(&self) -> String {
        format!("{}_{}", self.token, self.file)
    }

    fn to_document(&self) -> MongoDocument {
        doc! {
            "key": self.key(),
            "token": self.token.clone(),
            "file": self.file.clone(),
            "date": self.date.clone(),
        }
    }

    fn from_document(document: MongoDocument) -> Self {
        bson::from_bson::<Document>(bson::Bson::Document(document)).unwrap_or(Self::default())
    }
}
