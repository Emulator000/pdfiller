use std::fs;
use std::io::{Error, Write};

use actix_rt::blocking::BlockingError;
use actix_web::web;

use uuid::Uuid;

use crate::client;
use crate::config::ServiceConfig;

pub enum FileResult {
    Saved,
    NotSaved,
    Error(Error),
    BlockingError(BlockingError<Error>),
}

#[derive(Clone)]
pub struct File {
    config: ServiceConfig,
}

impl File {
    pub const PATH_COMPILED: &'static str = "compiled/";

    pub fn new(config: ServiceConfig) -> Self {
        Self { config }
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
            Some(file_name) => Some(format!("{}{}", Self::PATH_COMPILED, file_name)),
            None => None,
        }
    }

    pub fn generate_filepath<S: AsRef<str>>(&self, file_name: S) -> String {
        Self::file_path(self.config.temp.as_str(), file_name.as_ref())
    }

    pub async fn download_and_save<S: AsRef<str>>(&self, uri: S) -> Option<String> {
        let mut filepath = None;
        match client::get(uri.as_ref()).await {
            Some(pdf) => {
                let local_filepath = Self::file_path(self.config.temp.as_str(), "file.pdf");
                match Self::save_file(local_filepath.as_str(), pdf).await {
                    FileResult::Saved => {
                        filepath = Some(local_filepath.clone());
                    }
                    _ => {}
                }
            }
            None => {}
        }

        filepath
    }

    pub async fn save_file<S: AsRef<str>>(file_path: S, data: Vec<u8>) -> FileResult {
        match crystalsoft_utils::get_filepath::<&str>(file_path.as_ref()) {
            Some(filepath) => match fs::create_dir_all::<&str>(filepath.as_ref()) {
                Ok(_) => match web::block(|| fs::File::create::<String>(filepath)).await {
                    Ok(mut file) => match file.write_all(&data) {
                        Ok(_) => FileResult::Saved,
                        Err(e) => {
                            sentry::capture_error(&e);

                            FileResult::Error(e)
                        }
                    },
                    Err(e) => {
                        sentry::capture_error(&e);

                        FileResult::BlockingError(e)
                    }
                },
                Err(e) => {
                    sentry::capture_error(&e);

                    FileResult::Error(e)
                }
            },
            None => FileResult::NotSaved,
        }
    }
}
