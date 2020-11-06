pub mod local;
pub mod s3;

use std::io::Error;

use async_trait::async_trait;

use actix_rt::blocking::BlockingError;

use uuid::Uuid;

pub const PATH_COMPILED: &'static str = "compiled/";

pub enum FileResult {
    Saved,
    NotSaved,
    Error(Error),
    BlockingError(BlockingError<Error>),
}

#[async_trait]
pub trait FileProvider {
    fn file_path<S: AsRef<str>>(path: S, filename: S) -> String
    where
        Self: Sized,
    {
        format!(
            "{}{}{}",
            path.as_ref(),
            Uuid::new_v4().to_string(),
            sanitize_filename::sanitize(filename.as_ref())
        )
    }

    fn get_compiled_filepath<S: AsRef<str>>(file_path: S) -> Option<String>
    where
        Self: Sized,
    {
        match crystalsoft_utils::get_filename(file_path) {
            Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
            None => None,
        }
    }

    fn generate_filepath<S: AsRef<str>>(&self, file_name: S) -> String
    where
        Self: Sized;

    async fn download_and_save<S: AsRef<str>>(&self, uri: S) -> Option<String>
    where
        Self: Sized;

    async fn save_file<S: AsRef<str>>(&self, file_path: S, data: Vec<u8>) -> FileResult
    where
        Self: Sized;
}
