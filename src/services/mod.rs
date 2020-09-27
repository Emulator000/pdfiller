mod document;
mod filler;

use actix_web::web;

#[derive(Serialize)]
struct WsMessage {
    message: String,
}

#[derive(Serialize)]
struct WsError {
    error: String,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    document::config(cfg);
    filler::config(cfg);
}
