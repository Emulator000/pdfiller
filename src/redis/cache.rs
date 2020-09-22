use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use async_std::sync::RwLock;

use arc_swap::ArcSwap;

use crate::redis::models::Model;

type AnyObject = Box<dyn Any + Send + Sync>;
type CacheObject = Option<ArcSwap<AnyObject>>;
type CacheMap = HashMap<String, CacheObject>;
type HashCache = Arc<RwLock<CacheMap>>;

pub enum CacheResult {
    Ok,
    Error,
}

#[derive(Clone)]
pub struct Cache {
    models: HashCache,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(CacheMap::new())),
        }
    }

    pub async fn get<T: 'static + Model, S: AsRef<str>>(&self, key: S) -> Option<Option<Arc<T>>> {
        match self.models.read().await.get(key.as_ref()) {
            Some(object) => match object {
                Some(object) => Some(match object.load().downcast_ref::<Arc<T>>() {
                    Some(value) => Some(value.to_owned()),
                    None => None,
                }),
                None => Some(None),
            },
            None => None,
        }
    }

    pub async fn set<T: 'static + Model, S: AsRef<str>>(
        &self,
        key: S,
        value: Option<T>,
    ) -> CacheResult {
        match self.models.write().await.insert(
            key.as_ref().to_owned(),
            match value {
                Some(value) => Some(ArcSwap::new(Arc::new(
                    Box::new(Arc::new(value)) as AnyObject
                ))),
                None => None,
            },
        ) {
            Some(_) => CacheResult::Ok,
            None => CacheResult::Error,
        }
    }

    pub async fn del<S: AsRef<str>>(&self, key: S) -> CacheResult {
        match self.models.write().await.remove(key.as_ref()) {
            Some(_) => CacheResult::Ok,
            None => CacheResult::Error,
        }
    }
}
