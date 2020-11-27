#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate bson;
#[macro_use]
extern crate log;

mod client;
mod config;
mod data;
mod file;
mod mongo;
mod services;
mod utils;

use env_logger::Env;

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
use crate::mongo::wrapper::MongoWrapper;
use crate::mongo::MongoDB;

const API_VERSION: &str = "v1";

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

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
        MongoWrapper::new(MongoDB::new(&config.mongo).await),
    );

    info!(
        "Starting PDFIller API server at http://{}:{}...",
        config.server.bind_address, config.server.bind_port
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
