pub mod local;
pub mod s3;

use std::io::Error;

use async_trait::async_trait;

use actix_rt::blocking::BlockingError;

use uuid::Uuid;

use ::s3::S3Error;

use crate::client;

pub const PATH_COMPILED: &'static str = "compiled/";

pub enum FileResult {
    Saved,
    NotSaved,
    Error(Error),
    BlockingError(BlockingError<Error>),
    S3Error(S3Error),
}

#[derive(Debug)]
pub enum FileError {
    IoError(Error),
    S3Error(S3Error),
}

#[async_trait]
pub trait FileProvider: Send + Sync {
    fn generate_filepath(&self, file_name: &str) -> String {
        format!(
            "{}{}{}",
            self.base_path(),
            Uuid::new_v4().to_string(),
            sanitize_filename::sanitize(file_name)
        )
    }

    async fn download_and_save(&self, uri: &str) -> Option<String> {
        let mut filepath = None;
        match client::get(uri).await {
            Some(pdf) => {
                let remote_file_path = self.generate_filepath("file.pdf");
                match self.save(remote_file_path.as_str(), pdf).await {
                    FileResult::Saved => {
                        filepath = Some(remote_file_path.clone());
                    }
                    FileResult::Error(e) => {
                        sentry::capture_error(&e);
                    }
                    FileResult::S3Error(e) => {
                        sentry::capture_error(&e);
                    }
                    FileResult::BlockingError(e) => {
                        sentry::capture_error(&e);
                    }
                    _ => {}
                }
            }
            None => {}
        }

        filepath
    }

    async fn load(&self, file_path: &str) -> Result<Vec<u8>, FileError>;

    async fn save(&self, file_path: &str, data: Vec<u8>) -> FileResult;

    fn base_path(&self) -> &str;
}

pub fn get_compiled_filepath<S: AsRef<str>>(file_path: S) -> Option<String> {
    match crystalsoft_utils::get_filename(file_path) {
        Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
        None => None,
    }
}
