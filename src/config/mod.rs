use std::env;

use log::info;
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
    pub string: Option<String>,
    pub host: Option<String>,
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
            Ok(configuration) => {
                info!("\"{}\" loaded correctly.", path.as_ref());

                let configuration =
                    envsubst::substitute(configuration, &env::vars().collect()).unwrap();

                toml::from_str(&configuration).unwrap_or_else(|e| {
                    panic!(
                        "Error {:#?} loading this configuration: {:#?}",
                        e, configuration
                    )
                })
            }
            Err(e) => panic!("Couldn't open \"{}\", error: {:#?}", path.as_ref(), e),
        }
    }
}
