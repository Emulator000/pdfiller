use std::sync::Arc;

use async_std::sync::RwLock;
use bson::{doc, oid::ObjectId};
use futures_lite::StreamExt;
use log::{error, info};
use mongodb::error::Error as MongoDBError;
use mongodb::options::{ClientOptions, FindOptions};
use mongodb::{Client, Collection, Database};
use simple_cache::{Cache, CacheError};

use crate::config::MongoConfig;
use crate::mongo::models::Model;

pub mod models;
pub mod wrapper;

type MongoResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MongoDBError(MongoDBError),
    CacheError(CacheError),
}

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

    pub async fn insert<T: 'static + Model>(&self, model: T) -> MongoResult<()> {
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
                        .map_err(|e| Error::CacheError(e))?;
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

                Err(Error::MongoDBError(e))
            }
        }
    }

    #[allow(dead_code)]
    pub async fn update_one<T: 'static + Model>(&self, id: ObjectId, model: T) -> MongoResult<()> {
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

                Error::MongoDBError(e)
            })?;

        self.cache
            .insert::<T>(id, Some(model))
            .map_err(|e| Error::CacheError(e))?;

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
                error!(
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
        // self.cache
        //     .get::<T, _>(&id)
        //     .map(|value| {
        //         if let Some(value) = value {
        //             value
        //         } else {
        //             match self
        //                 .get_collection(T::name())
        //                 .await
        //                 .find_one(
        //                     doc! {
        //                         "_id": id.clone(),
        //                     },
        //                     None,
        //                 )
        //                 .await
        //             {
        //                 Ok(result) => match result {
        //                     Some(document) => {
        //                         self.cache
        //                             .insert::<T>(
        //                                 id.clone(),
        //                                 Some(
        //                                     T::from_document(document)
        //                                         .unwrap_or_else(|_| T::default()),
        //                                 ),
        //                             )
        //                             .map_err(|e| Error::CacheError(e))?;
        //                     }
        //                     None => {
        //                         self.cache
        //                             .insert::<T>(id.clone(), None)
        //                             .map_err(|e| Error::CacheError(e))?;
        //                     }
        //                 },
        //                 Err(e) => {
        //                     error!("Error getting {} with key {}: {:#?}", T::name(), &id, e);
        //
        //                     sentry::capture_error(&e);
        //
        //                     self.cache
        //                         .insert::<T>(id.clone(), None)
        //                         .map_err(|e| Error::CacheError(e))?;
        //                 }
        //             }
        //
        //             self.cache
        //                 .get::<T, _>(&id)
        //                 .map(|value| if let Some(value) = value { value } else { None })
        //                 .ok()
        //         }
        //     })
        //     .ok()

        if let Ok(result) = self.cache.get::<T, _>(&id) {
            if let Some(value) = result {
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
                            let _ = self.cache.insert::<T>(
                                id.clone(),
                                Some(T::from_document(document).unwrap_or_else(|_| T::default())),
                            );
                        }
                        None => {
                            let _ = self.cache.insert::<T>(id.clone(), None);
                        }
                    },
                    Err(e) => {
                        error!("Error getting {} with key {}: {:#?}", T::name(), &id, e);

                        sentry::capture_error(&e);

                        let _ = self.cache.insert::<T>(id.clone(), None);
                    }
                }

                if let Ok(result) = self.cache.get::<T, _>(&id) {
                    if let Some(value) = result {
                        value
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub async fn delete_one<T: Model>(&self, id: ObjectId) -> MongoResult<()> {
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
                error!("Error getting {} with key {}: {:#?}", T::name(), &id, e);

                sentry::capture_error(&e);

                e
            })
            .map_err(|e| Error::MongoDBError(e))?;

        self.cache.remove(&id).map_err(|e| Error::CacheError(e))?;

        Ok(())
    }
}
