use std::env;

use serde::Deserialize;

use toml;

use crate::utils::read_file_string;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub redis: RedisConfig,
    pub sentry: SentryConfig,
}

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub bind_port: u32,
}

#[derive(Clone, Deserialize)]
pub struct RedisConfig {
    pub host: String,
    pub port: Option<u32>,
}

#[derive(Clone, Deserialize)]
pub struct SentryConfig {
    pub dsn: String,
}

impl Config {
    pub fn new<S: AsRef<str>>(path: S) -> Self {
        match read_file_string(path.as_ref()) {
            Some(configuration) => toml::from_str(
                &envsubst::substitute(configuration, &env::vars().collect()).unwrap(),
            )
            .unwrap(),
            None => panic!("Couldn't open {} file.", path.as_ref()),
        }
    }
}
