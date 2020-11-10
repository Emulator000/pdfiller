use serde::de::DeserializeOwned;
use serde::Serialize;

use simple_cache::CacheItem;

pub mod document;

pub trait Model: CacheItem + Send + Sync + Unpin + Serialize + DeserializeOwned {
    fn name() -> &'static str;

    fn key(&self) -> String;

    fn prefix() -> String {
        format!("{}", Self::name())
    }

    fn model_key<T: Model, S: AsRef<str>>(value: Option<S>) -> String {
        if let Some(value) = value {
            format!("{}_{}", T::prefix(), value.as_ref())
        } else {
            format!("{}_*", T::prefix())
        }
    }
}
