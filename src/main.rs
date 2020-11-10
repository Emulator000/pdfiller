#[macro_use]
extern crate serde_derive;

mod client;
mod config;
mod data;
mod file;
mod logger;
mod redis;
mod services;
mod utils;

use actix_web::{
    middleware::{
        normalize::{NormalizePath, TrailingSlash},
        Logger,
    },
    web, App, HttpServer,
};

use crate::config::Config;
use crate::data::Data;
use crate::file::local::Local;
use crate::file::s3::S3;
use crate::redis::wrapper::RedisWrapper;
use crate::redis::Redis;

const API_VERSION: &'static str = "v1";

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let config = Config::new("config/config.toml");

    if let Some(sentry) = config.sentry {
        let _guard = sentry::init(sentry.dsn);
    }

    let data = Data::new(
        if config.service.filesystem == "local" {
            Box::new(Local::new(config.service.clone()))
        } else {
            Box::new(S3::new(config.service.clone()))
        },
        RedisWrapper::new(Redis::new(&config.mongo).await),
    );

    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(Logger::default())
            .service(web::scope(&format!("/api/{}", API_VERSION)).configure(services::config))
    })
    .bind(format!(
        "{}:{}",
        config.server.bind_address, config.server.bind_port
    ))?
    .run()
    .await
}
