pub mod models;
pub mod wrapper;

use std::sync::Arc;

use async_std::sync::RwLock;

use futures_lite::StreamExt;

use mongodb::error::Error;
use mongodb::options::{ClientOptions, FindOptions};
use mongodb::{Client, Collection, Database};

use simple_cache::Cache;

use crate::config::MongoConfig;
use crate::logger::Logger;
use crate::mongo::models::Model;

#[derive(Clone)]
pub struct MongoDB {
    database: Arc<RwLock<Database>>,
    cache: Cache<String>,
}

impl MongoDB {
    const MONGDB_STR: &'static str = "mongodb";
    const DEFAULT_PORT: u16 = 27017;

    pub async fn new(config: &MongoConfig) -> Self {
        let connection_str = format!(
            "{}:{}",
            config.host,
            config.port.unwrap_or(Self::DEFAULT_PORT)
        );

        let options = ClientOptions::parse(
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
            .as_str(),
        )
        .await
        .unwrap_or_else(|e| panic!("Error {:#?} creating connection to {}", e, config.host));

        let client = Client::with_options(options)
            .unwrap_or_else(|e| panic!("Error {:#?} connecting to {}", e, config.host));

        MongoDB {
            database: Arc::new(RwLock::new(client.database(config.db_name.as_str()))),
            cache: Cache::new(),
        }
    }

    async fn get_collection<S: AsRef<str>>(&self, name: S) -> Collection {
        self.database.write().await.collection(name.as_ref())
    }

    pub async fn insert<T: 'static + Model>(&self, model: T) -> Result<(), Error> {
        let key = model.key();
        self.get_collection(T::name())
            .await
            .insert_one(model.to_document(), None)
            .await
            .map_err(|e| {
                sentry::capture_error(&e);

                Logger::log(format!(
                    "Error getting {} with key {}: {:#?}",
                    T::name(),
                    &key,
                    e
                ));

                e
            })?;

        self.cache.insert::<T>(key.clone(), Some(model)).await;

        Ok(())
    }

    pub async fn update_one<T: 'static + Model>(&self, model: T) -> Result<(), Error> {
        let key = model.key();
        self.get_collection(T::name())
            .await
            .update_one(
                doc! {
                    "key": key.clone(),
                },
                model.to_document(),
                None,
            )
            .await
            .map_err(|e| {
                sentry::capture_error(&e);

                Logger::log(format!(
                    "Error getting {} with key {}: {:#?}",
                    T::name(),
                    &key,
                    e
                ));

                e
            })?;

        self.cache.insert::<T>(key.clone(), Some(model)).await;

        Ok(())
    }

    pub async fn get<T: Model, S: AsRef<str>>(
        &self,
        key_value: Option<(S, S)>,
        sort_by: S,
    ) -> Option<Vec<T>> {
        let filter = if let Some(ref key_value) = key_value {
            Some(doc! {
                key_value.0.as_ref(): key_value.1.as_ref(),
            })
        } else {
            None
        };

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
                    match document {
                        Ok(document) => {
                            results.push(T::from_document(document));
                        }
                        Err(_) => {}
                    }
                }

                Some(results)
            }
            Err(e) => {
                sentry::capture_error(&e);

                Logger::log(format!(
                    "Error getting keys with pattern {:#?}: {:#?}",
                    if let Some(key_value) = key_value {
                        format!("{}, {}", key_value.0.as_ref(), key_value.1.as_ref())
                    } else {
                        "[empty]".into()
                    },
                    e
                ));

                None
            }
        }
    }

    #[allow(dead_code)]
    pub async fn get_one<T: 'static + Model, S: AsRef<str>>(&self, key: S) -> Option<Arc<T>> {
        if let Some(value) = self.cache.get::<T, _>(key.as_ref()).await {
            value
        } else {
            match self
                .get_collection(T::name())
                .await
                .find_one(
                    doc! {
                        "key": key.as_ref().to_owned(),
                    },
                    None,
                )
                .await
            {
                Ok(result) => match result {
                    Some(document) => {
                        self.cache
                            .insert::<T>(key.as_ref().to_owned(), Some(T::from_document(document)))
                            .await;
                    }
                    None => {
                        self.cache.insert::<T>(key.as_ref().to_owned(), None).await;
                    }
                },
                Err(e) => {
                    sentry::capture_error(&e);

                    Logger::log(format!(
                        "Error getting {} with key {}: {:#?}",
                        T::name(),
                        key.as_ref(),
                        e
                    ));

                    self.cache.insert::<T>(key.as_ref().to_owned(), None).await;
                }
            }

            self.cache.get::<T, _>(key.as_ref()).await.unwrap()
        }
    }

    #[allow(dead_code)]
    pub async fn delete_one<T: Model>(&self, model: T) -> Result<(), Error> {
        let key = model.key();
        self.get_collection(T::name())
            .await
            .delete_one(
                doc! {
                    "key": key.clone(),
                },
                None,
            )
            .await
            .map_err(|e| {
                sentry::capture_error(&e);

                Logger::log(format!(
                    "Error getting {} with key {}: {:#?}",
                    T::name(),
                    &key,
                    e
                ));

                e
            })?;

        self.cache.remove(&key).await;

        Ok(())
    }
}
