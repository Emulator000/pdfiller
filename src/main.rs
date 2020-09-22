#[macro_use]
extern crate serde_derive;

mod config;
mod data;
mod logger;
mod redis;
mod services;

use actix_web::{
    middleware::{
        normalize::{NormalizePath, TrailingSlash},
        Logger,
    },
    web, App, HttpServer,
};

use crate::config::Config;
use crate::data::Data;
use crate::redis::wrapper::RedisWrapper;
use crate::redis::Redis;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let config = Config::new("config/config.toml");

    let _guard = sentry::init(config.sentry.dsn);

    let data = Data::new(RedisWrapper::new(Redis::new(&config.redis).await));

    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(Logger::default())
            .service(web::scope("/api/v1").configure(services::config))
    })
    .bind(format!(
        "{}:{}",
        config.server.bind_address, config.server.bind_port
    ))?
    .run()
    .await
}
