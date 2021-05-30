use std::fs;
use std::io::Write;

use async_trait::async_trait;

use actix_web::web;

use crate::config::ServiceConfig;
use crate::file::{FileError, FileProvider, FileResult};

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
    async fn load(&self, file_path: &str) -> FileResult<Vec<u8>> {
        crystalsoft_utils::read_file_buf(file_path).map_err(FileError::IoError)
    }

    async fn save(&self, file_path: &str, data: Vec<u8>) -> FileResult<()> {
        match crystalsoft_utils::get_filepath(file_path) {
            Some(path) => match fs::create_dir_all(path) {
                Ok(_) => {
                    let file_path: String = file_path.into();
                    match web::block(|| fs::File::create(file_path)).await {
                        Ok(Ok(mut file)) => match file.write_all(&data) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(FileError::IoError(e)),
                        },
                        Ok(Err(e)) => Err(FileError::IoError(e)),
                        Err(e) => Err(FileError::BlockingError(e)),
                    }
                }
                Err(e) => Err(FileError::IoError(e)),
            },
            None => Err(FileError::NotSaved),
        }
    }

    fn base_path(&self) -> &str {
        self.config.path.as_str()
    }
}
