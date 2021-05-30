use std::sync::Arc;

use async_std::sync::RwLock;
use bson::oid::ObjectId;
use futures_lite::StreamExt;
use mongodb::error::Error;
use mongodb::options::{ClientOptions, FindOptions};
use mongodb::{Client, Collection, Database};
use simple_cache::Cache;

use crate::config::MongoConfig;
use crate::mongo::models::Model;

pub mod models;
pub mod wrapper;

#[derive(Clone)]
pub struct MongoDB {
    database: Arc<RwLock<Database>>,
    cache: Cache<ObjectId>,
}

impl MongoDB {
    const MONGDB_STR: &'static str = "mongodb";
    const DEFAULT_PORT: u16 = 27017;

    pub async fn new(config: &MongoConfig) -> Self {
        let connection_str = match (config.string.as_ref(), config.host.as_ref()) {
            (Some(connection_str), _) => String::from(connection_str),
            (None, Some(host)) => {
                let connection_str =
                    format!("{}:{}", host, config.port.unwrap_or(Self::DEFAULT_PORT));

                if let Some(ref user) = config.user {
                    format!(
                        "{}://{}:{}@{}",
                        Self::MONGDB_STR,
                        user,
                        config.password.as_deref().unwrap_or(""),
                        connection_str
                    )
                } else {
                    format!("{}://{}", Self::MONGDB_STR, connection_str)
                }
            }
            (None, None) => {
                panic!("Missing configuration for MongoDB")
            }
        };

        let options = ClientOptions::parse(connection_str.as_str())
            .await
            .unwrap_or_else(|e| panic!("Error {:#?} creating connection to MongoDB", e));

        let client = Client::with_options(options)
            .unwrap_or_else(|e| panic!("Error {:#?} connecting to MongoDB", e));

        MongoDB {
            database: Arc::new(RwLock::new(client.database(config.db_name.as_str()))),
            cache: Cache::new(),
        }
    }

    async fn get_collection<S: AsRef<str>>(&self, name: S) -> Collection {
        self.database.write().await.collection(name.as_ref())
    }

    pub async fn insert<T: 'static + Model>(&self, model: T) -> Result<(), Error> {
        match self
            .get_collection(T::name())
            .await
            .insert_one(model.to_document(), None)
            .await
            .map_err(|e| {
                info!(
                    "Error getting {} with model {}: {:#?}",
                    T::name(),
                    model.debug(),
                    e
                );

                sentry::capture_error(&e);

                e
            }) {
            Ok(result) => {
                if let Some(object_id) = result.inserted_id.as_object_id() {
                    self.cache
                        .insert::<T>(object_id.to_owned(), Some(model))
                        .await;
                }

                Ok(())
            }
            Err(e) => {
                info!(
                    "Error getting {} with model {}: {:#?}",
                    T::name(),
                    model.debug(),
                    e
                );

                sentry::capture_error(&e);

                Err(e)
            }
        }
    }

    #[allow(dead_code)]
    pub async fn update_one<T: 'static + Model>(
        &self,
        id: ObjectId,
        model: T,
    ) -> Result<(), Error> {
        self.get_collection(T::name())
            .await
            .update_one(
                doc! {
                    "_id": id.clone(),
                },
                model.to_document(),
                None,
            )
            .await
            .map_err(|e| {
                info!("Error getting {} with key {}: {:#?}", T::name(), &id, e);

                sentry::capture_error(&e);

                e
            })?;

        self.cache.insert::<T>(id, Some(model)).await;

        Ok(())
    }

    pub async fn get<T: Model, S: AsRef<str>>(
        &self,
        key_value: Option<(S, S)>,
        sort_by: S,
    ) -> Option<Vec<T>> {
        let filter = key_value.as_ref().map(|key_value| {
            doc! {
                key_value.0.as_ref(): key_value.1.as_ref(),
            }
        });

        match self
            .get_collection(T::name())
            .await
            .find(
                filter,
                FindOptions::builder()
                    .sort(doc! { sort_by.as_ref(): 1 })
                    .build(),
            )
            .await
        {
            Ok(mut cursor) => {
                let mut results = Vec::new();
                while let Some(document) = cursor.next().await {
                    if let Ok(document) = document {
                        results.push(T::from_document(document).unwrap_or_else(|_| T::default()));
                    }
                }

                Some(results)
            }
            Err(e) => {
                info!(
                    "Error getting keys with pattern {:#?}: {:#?}",
                    if let Some(ref key_value) = key_value {
                        format!("{}, {}", key_value.0.as_ref(), key_value.1.as_ref())
                    } else {
                        "[empty]".into()
                    },
                    e
                );

                sentry::capture_error(&e);

                None
            }
        }
    }

    #[allow(dead_code)]
    pub async fn get_one<T: 'static + Model, S: AsRef<str>>(&self, id: ObjectId) -> Option<Arc<T>> {
        if let Some(value) = self.cache.get::<T, _>(&id).await {
            value
        } else {
            match self
                .get_collection(T::name())
                .await
                .find_one(
                    doc! {
                        "_id": id.clone(),
                    },
                    None,
                )
                .await
            {
                Ok(result) => match result {
                    Some(document) => {
                        self.cache
                            .insert::<T>(
                                id.clone(),
                                Some(T::from_document(document).unwrap_or_else(|_| T::default())),
                            )
                            .await;
                    }
                    None => {
                        self.cache.insert::<T>(id.clone(), None).await;
                    }
                },
                Err(e) => {
                    info!("Error getting {} with key {}: {:#?}", T::name(), &id, e);

                    sentry::capture_error(&e);

                    self.cache.insert::<T>(id.clone(), None).await;
                }
            }

            self.cache.get::<T, _>(&id).await.unwrap()
        }
    }

    #[allow(dead_code)]
    pub async fn delete_one<T: Model>(&self, id: ObjectId) -> Result<(), Error> {
        self.get_collection(T::name())
            .await
            .delete_one(
                doc! {
                    "_id": id.clone(),
                },
                None,
            )
            .await
            .map_err(|e| {
                info!("Error getting {} with key {}: {:#?}", T::name(), &id, e);

                sentry::capture_error(&e);

                e
            })?;

        self.cache.remove(&id).await;

        Ok(())
    }
}
