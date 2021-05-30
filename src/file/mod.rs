pub mod local;
pub mod s3;

use std::fmt;
use std::io::Error;

use ::s3::S3Error;
use actix_web::error::BlockingError;
use async_trait::async_trait;
use serde::de::StdError;
use uuid::Uuid;

use crate::client;

pub const PATH_COMPILED: &str = "compiled/";

pub type FileResult<T> = Result<T, FileError>;

#[derive(Debug)]
pub enum FileError {
    NotSaved,
    BlockingError(BlockingError<Error>),
    IoError(Error),
    S3Error(S3Error),
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSaved => {
                write!(f, "File couldn't be saved")
            }
            FileError::BlockingError(e) => {
                write!(f, "{:#?}", e)
            }
            FileError::IoError(e) => {
                write!(f, "{:#?}", e)
            }
            FileError::S3Error(e) => {
                write!(f, "{:#?}", e)
            }
        }
    }
}

impl StdError for FileError {}

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

    fn generate_compiled_filepath(&self, file_path: &str) -> Option<String> {
        crystalsoft_utils::get_filename(file_path)
            .map(|file_name| format!("{}{}{}", self.base_path(), PATH_COMPILED, file_name))
    }

    async fn download_and_save(&self, uri: &str) -> Option<String> {
        let mut filepath = None;
        if let Some(pdf) = client::get(uri).await {
            let remote_file_path = self.generate_filepath("file.pdf");
            match self.save(remote_file_path.as_str(), pdf).await {
                Ok(_) => {
                    filepath = Some(remote_file_path.clone());
                }
                Err(FileError::IoError(e)) => {
                    sentry::capture_error(&e);
                }
                Err(FileError::S3Error(e)) => {
                    sentry::capture_error(&e);
                }
                Err(FileError::BlockingError(e)) => {
                    sentry::capture_error(&e);
                }
                _ => {}
            }
        }

        filepath
    }

    async fn load(&self, file_path: &str) -> FileResult<Vec<u8>>;

    async fn save(&self, file_path: &str, data: Vec<u8>) -> FileResult<()>;

    fn base_path(&self) -> &str;
}
