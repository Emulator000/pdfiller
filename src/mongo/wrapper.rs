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
    pub async fn get_all<T: 'static + Model, S: AsRef<str>>(&self, sort_by: S) -> Option<Vec<T>> {
        self.mongo.get::<T, _>(None, sort_by).await
    }

    pub async fn get_all_by<T: 'static + Model, S: AsRef<str>>(
        &self,
        key: S,
        value: S,
        sort_by: S,
    ) -> Option<Vec<T>> {
        self.mongo.get::<T, _>(Some((key, value)), sort_by).await
    }

    pub async fn create<T: 'static + Model>(&self, model: T) -> Result<(), Error> {
        self.mongo.insert::<T>(model).await
    }
}
