mod client;
mod config;
mod data;
mod file;
mod mongo;
mod services;
mod utils;

use std::io::Write;

use actix_web::{
    middleware::{
        normalize::{NormalizePath, TrailingSlash},
        Logger,
    },
    web, App, HttpServer,
};
use chrono::Local as ChronoLocal;
use clap::{App as ClapApp, Arg};
use env_logger::Env;
use log::info;

use crate::config::Config;
use crate::data::Data;
use crate::file::local::Local;
use crate::file::s3::S3;
use crate::mongo::wrapper::MongoWrapper;
use crate::mongo::MongoDB;

const API_VERSION: &str = "v1";

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {} - {}",
                ChronoLocal::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or("main"),
                record.args()
            )
        })
        .init();

    let name = "PDFIller";

    let matches = ClapApp::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .name(name)
        .author("Dario Cancelliere <dario.cancelliere@facile.it>")
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .required(false)
                .takes_value(true)
                .default_value("config")
                .help("Base config file path"),
        )
        .get_matches();

    info!(
        "{} v{} by Dario Cancelliere",
        name,
        env!("CARGO_PKG_VERSION")
    );
    info!("{}", env!("CARGO_PKG_DESCRIPTION"));
    info!("");

    let config = Config::new(&format!(
        "{}/config.toml",
        matches.value_of("path").unwrap()
    ));

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
