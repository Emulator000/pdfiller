use std::fs;
use std::io::Write;

use async_trait::async_trait;

use actix_web::web;

use crate::client;
use crate::config::ServiceConfig;
use crate::file::{FileProvider, FileResult};

#[derive(Clone)]
pub struct Local {
    config: ServiceConfig,
}

impl Local {
    pub fn new(config: ServiceConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl FileProvider for Local {
    fn generate_filepath<S: AsRef<str>>(&self, file_name: S) -> String {
        Self::file_path(self.config.path.as_str(), file_name.as_ref())
    }

    async fn download_and_save<S: AsRef<str>>(&self, uri: S) -> Option<String> {
        let mut filepath = None;
        match client::get(uri.as_ref()).await {
            Some(pdf) => {
                let local_filepath = Self::file_path(self.config.path.as_str(), "file.pdf");
                match self.save_file(local_filepath.as_str(), pdf).await {
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

    async fn save_file<S: AsRef<str>>(&self, file_path: S, data: Vec<u8>) -> FileResult {
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
