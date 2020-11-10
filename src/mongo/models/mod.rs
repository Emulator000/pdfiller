use serde::de::DeserializeOwned;
use serde::Serialize;

use mongodb::bson::Document as MongoDocument;

use simple_cache::CacheItem;

pub mod document;

pub trait Model: CacheItem + Send + Sync + Unpin + Serialize + DeserializeOwned {
    fn name() -> &'static str;

    fn prefix() -> String {
        format!("{}", Self::name())
    }

    fn debug(&self) -> String;

    fn to_document(&self) -> MongoDocument;

    fn from_document(document: MongoDocument) -> Self;
}
