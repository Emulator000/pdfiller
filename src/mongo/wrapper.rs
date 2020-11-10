use async_std::sync::Arc;

use mongodb::error::Error;

use crate::mongo::models::Model;
use crate::mongo::MongoDB;

#[derive(Clone)]
pub struct MongoWrapper {
    mongo: MongoDB,
}

impl MongoWrapper {
    pub fn new(mongo: MongoDB) -> Self {
        Self { mongo }
    }

    /// Generic
    pub async fn get_all<T: 'static + Model>(&self) -> Option<Vec<T>> {
        self.mongo.get::<T, &str>(None).await
    }

    pub async fn get_all_by<T: 'static + Model, S: AsRef<str>>(&self, value: S) -> Option<Vec<T>> {
        self.mongo.get::<T, _>(Some(value)).await
    }

    #[allow(dead_code)]
    pub async fn get<T: 'static + Model, S: AsRef<str>>(&self, value: S) -> Option<Arc<T>> {
        self.mongo.get_one::<T, _>(value).await
    }

    pub async fn create<T: 'static + Model>(&self, model: T) -> Result<(), Error> {
        self.mongo.insert::<T>(model).await
    }

    #[allow(dead_code)]
    pub async fn update<T: 'static + Model, S: AsRef<str>>(&self, model: T) -> Result<(), Error> {
        self.mongo.update_one::<T>(model).await
    }

    #[allow(dead_code)]
    pub async fn delete<T: Model, S: AsRef<str>>(&self, model: T) -> Result<(), Error> {
        self.mongo.delete_one::<T>(model).await
    }
}
