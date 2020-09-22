use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;

use toml;

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
        let path = Path::new(path.as_ref());
        let display = path.display();

        let mut file = match File::open(&path) {
            Err(why) => panic!("Couldn't open {}: {}", display, why),
            Ok(file) => file,
        };

        let mut configuration = String::new();
        match file.read_to_string(&mut configuration) {
            Err(why) => panic!("Couldn't read {}: {}", display, why),
            Ok(_) => println!("{} loaded correctly.", display),
        }

        toml::from_str(&envsubst::substitute(configuration, &env::vars().collect()).unwrap())
            .unwrap()
    }
}
