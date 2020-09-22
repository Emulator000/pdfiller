mod handler;

use std::str;

use actix_web::{post, web, HttpResponse, Responder};

use serde::Deserialize;
use serde_json::Value;

use crate::data::Data;
use crate::redis::models::document::Document;
use crate::services::WsError;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(compile_documents);
}

#[post("/compile/{token}")]
pub async fn compile_documents(
    bytes: web::Bytes,
    data: web::Data<Data>,
    token: web::Path<String>,
) -> impl Responder {
    match str::from_utf8(&bytes) {
        Ok(body) => match serde_json::from_str::<Value>(body) {
            Ok(values) => {
                if let Some(value) = values.get("data") {
                    match <handler::PDFillerMap>::deserialize(value) {
                        Ok(map) => {
                            if let Some(documents) =
                                data.redis.get_all_by::<Document, _>(&token.0).await
                            {
                                if !documents.is_empty() {
                                    handler::compile_documents(&map, &documents)
                                } else {
                                    HttpResponse::NotFound().json(WsError {
                                        error: "No documents found for this token!".into(),
                                    })
                                }
                            } else {
                                HttpResponse::NotFound().json(WsError {
                                    error: "No documents found for this token!".into(),
                                })
                            }
                        }
                        Err(e) => HttpResponse::BadRequest().json(WsError {
                            error: format!("Not a valid PDFiller request: {:#?}", e),
                        }),
                    }
                } else {
                    HttpResponse::BadRequest().json(WsError {
                        error: "Not a valid PDFiller request.".into(),
                    })
                }
            }
            Err(e) => HttpResponse::BadRequest().json(WsError {
                error: format!("Couldn't decode the body as JSON: {:#?}", e),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(WsError {
            error: format!("Error decoding the body: {:#?}", e),
        }),
    }
}
