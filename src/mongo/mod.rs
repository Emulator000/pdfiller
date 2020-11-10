pub mod models;
pub mod wrapper;

use std::sync::Arc;

use async_std::sync::{RwLock, RwLockWriteGuard};

use mongodb::error::Error;
use mongodb::options::{ClientOptions, StreamAddress};
use mongodb::{Client, Database};

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
    pub async fn new(config: &MongoConfig) -> Self {
        let options = ClientOptions::builder()
            .hosts(vec![StreamAddress {
                hostname: config.host.clone(),
                port: config.port,
            }])
            .build();

        let client = Client::with_options(options)
            .unwrap_or_else(|e| panic!("Error {:#?} connecting to {}", e, config.host));

        MongoDB {
            database: Arc::new(RwLock::new(client.database(config.db_name.as_str()))),
            cache: Cache::new(),
        }
    }

    pub async fn insert<T: Model>(&self, model: T) -> Result<(), Error> {
        unimplemented!();
        // redis::cmd("SET")
        //     .arg(&[
        //         model.key(),
        //         serde_json::to_string(&model).unwrap_or(String::new()),
        //     ])
        //     .query_async(&mut *self.database().await)
        //     .await
    }

    pub async fn update_one<T: 'static + Model>(&self, model: T) -> Result<(), Error> {
        unimplemented!();
        // let key = model.key();
        // let result = redis::cmd("SET")
        //     .arg(&[
        //         &key,
        //         &serde_json::to_string(&model).unwrap_or(String::new()),
        //     ])
        //     .query_async(&mut *self.database().await)
        //     .await;
        //
        // if result.is_ok() {
        //     self.cache.insert::<T>(key, Some(model)).await;
        // }
        //
        // result
    }

    // ToDo: find a way to introduce a cache here!
    pub async fn get<T: Model, S: AsRef<str>>(&self, value: Option<S>) -> Option<Vec<T>> {
        unimplemented!();
        // let key = if let Some(value) = value {
        //     format!("{}_*", T::model_key::<T, S>(Some(value)))
        // } else {
        //     T::model_key::<T, S>(None)
        // };
        //
        // let connection = &mut *self.database().await;
        // match redis::cmd("KEYS")
        //     .arg(&key)
        //     .query_async::<_, Vec<String>>(connection)
        //     .await
        // {
        //     Ok(res) => {
        //         let mut results = Vec::new();
        //         for key in res {
        //             match redis::cmd("GET")
        //                 .arg(&key)
        //                 .query_async::<_, String>(connection)
        //                 .await
        //             {
        //                 Ok(res) => match serde_json::from_str::<T>(&res) {
        //                     Ok(res) => {
        //                         results.push(res);
        //                     }
        //                     _ => {}
        //                 },
        //                 Err(e) => {
        //                     sentry::capture_error(&e);
        //
        //                     Logger::log(format!(
        //                         "Error getting {} with key {}: {:#?}",
        //                         T::name(),
        //                         &key,
        //                         e
        //                     ));
        //                 }
        //             }
        //         }
        //
        //         Some(results)
        //     }
        //     Err(e) => {
        //         sentry::capture_error(&e);
        //
        //         Logger::log(format!(
        //             "Error getting keys with pattern {}: {:#?}",
        //             &key, e
        //         ));
        //
        //         None
        //     }
        // }
    }

    #[allow(dead_code)]
    pub async fn get_one<T: 'static + Model, S: AsRef<str>>(&self, value: S) -> Option<Arc<T>> {
        unimplemented!();
        // let key = T::model_key::<T, S>(Some(value));
        // if let Some(value) = self.cache.get::<T, _>(&key).await {
        //     value
        // } else {
        //     match redis::cmd("GET")
        //         .arg(&key)
        //         .query_async::<_, String>(&mut *self.database().await)
        //         .await
        //     {
        //         Ok(res) => match serde_json::from_str::<T>(&res) {
        //             Ok(res) => {
        //                 self.cache.insert::<T>(key.clone(), Some(res)).await;
        //             }
        //             _ => {}
        //         },
        //         Err(e) => {
        //             sentry::capture_error(&e);
        //
        //             Logger::log(format!(
        //                 "Error getting {} with key {}: {:#?}",
        //                 T::name(),
        //                 &key,
        //                 e
        //             ));
        //
        //             self.cache.insert::<T>(key.clone(), None).await;
        //         }
        //     };
        //
        //     self.cache.get::<T, _>(&key).await.unwrap()
        // }
    }

    #[allow(dead_code)]
    pub async fn delete_one<T: Model>(&self, model: T) -> Result<(), Error> {
        unimplemented!();
        // let key = model.key();
        // let result = redis::cmd("DEL")
        //     .arg(&key)
        //     .query_async(&mut *self.database().await)
        //     .await;
        //
        // match &result {
        //     Ok(_) => {
        //         self.cache.remove(&key).await;
        //     }
        //     Err(e) => {
        //         sentry::capture_error(e);
        //
        //         Logger::log(format!(
        //             "Error deleting {} with key {}: {:#?}",
        //             T::name(),
        //             &key,
        //             e
        //         ));
        //     }
        // };
        //
        // result
    }
}
