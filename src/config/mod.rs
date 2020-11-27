use std::env;

use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub service: ServiceConfig,
    pub server: ServerConfig,
    pub mongo: MongoConfig,
    pub sentry: Option<SentryConfig>,
}

#[derive(Clone, Deserialize)]
pub struct ServiceConfig {
    pub filesystem: String,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub path: String,
}

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub bind_port: u32,
}

#[derive(Clone, Deserialize)]
pub struct MongoConfig {
    pub host: String,
    pub port: Option<u16>,
    pub db_name: String,
    pub user: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct SentryConfig {
    pub dsn: String,
}

impl Config {
    pub fn new<S: AsRef<str>>(path: S) -> Self {
        match crystalsoft_utils::read_file_string(path.as_ref()) {
            Ok(configuration) => toml::from_str(
                &envsubst::substitute(configuration, &env::vars().collect()).unwrap(),
            )
            .unwrap(),
            Err(e) => panic!("Couldn't open {} file: {:#?}", path.as_ref(), e),
        }
    }
}
