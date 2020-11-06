use async_trait::async_trait;

use s3::creds::Credentials;
use s3::Bucket;

use crate::config::ServiceConfig;
use crate::file::{FileError, FileProvider, FileResult};

#[derive(Clone)]
pub struct S3 {
    config: ServiceConfig,
    bucket: Bucket,
}

impl S3 {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            bucket: Bucket::new(
                config.s3_bucket.as_deref().unwrap(),
                config.s3_region.as_deref().unwrap().parse().unwrap(),
                Credentials::new(
                    config.s3_access_key.as_deref(),
                    config.s3_secret_key.as_deref(),
                    None,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap(),
            config,
        }
    }
}

#[async_trait]
impl FileProvider for S3 {
    async fn load(&self, file_path: &str) -> Result<Vec<u8>, FileError> {
        match self.bucket.get_object(file_path).await {
            Ok((data, _code)) => Ok(data),
            Err(e) => Err(FileError::S3Error(e)),
        }
    }

    async fn save(&self, file_path: &str, data: Vec<u8>) -> FileResult {
        match self.bucket.put_object(file_path, &data).await {
            Ok((_data, _code)) => FileResult::Saved,
            Err(e) => {
                sentry::capture_error(&e);

                FileResult::S3Error(e)
            }
        }
    }

    fn base_path(&self) -> &str {
        self.config.path.as_str()
    }
}
