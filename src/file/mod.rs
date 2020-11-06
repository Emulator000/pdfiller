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
pub trait FileProvider: Send + Sync {
    fn generate_filepath(&self, file_name: &str) -> String;

    async fn download_and_save(&self, uri: &str) -> Option<String>;

    async fn save_file(&self, file_path: &str, data: Vec<u8>) -> FileResult;
}

pub fn file_path<S: AsRef<str>>(path: S, filename: S) -> String {
    format!(
        "{}{}{}",
        path.as_ref(),
        Uuid::new_v4().to_string(),
        sanitize_filename::sanitize(filename.as_ref())
    )
}

pub fn get_compiled_filepath<S: AsRef<str>>(file_path: S) -> Option<String> {
    match crystalsoft_utils::get_filename(file_path) {
        Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
        None => None,
    }
}
