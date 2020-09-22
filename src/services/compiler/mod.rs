mod handler;

use std::str;

use actix_web::dev::BodyEncoding;
use actix_web::http::ContentEncoding;
use actix_web::{post, web, HttpResponse, Responder};

use serde::Deserialize;
use serde_json::Value;

use crate::data::Data;
use crate::redis::models::document::Document;
use crate::services::compiler::handler::{
    zip_compiled_documents, HandlerCompilerResult, ZipCompilerResult,
};
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
                                    match handler::compile_documents(&map, &documents) {
                                        HandlerCompilerResult::Success => {
                                            match zip_compiled_documents(&documents) {
                                                ZipCompilerResult::Success(bytes) => {
                                                    HttpResponse::Ok()
                                                        .encoding(ContentEncoding::Identity)
                                                        .content_type("application/zip")
                                                        .header("accept-ranges", "bytes")
                                                        .header(
                                                            "content-disposition",
                                                            "attachment; filename=\"pdfs.zip\"",
                                                        )
                                                        .body(bytes)
                                                }
                                                ZipCompilerResult::Error(message) => {
                                                    HttpResponse::InternalServerError()
                                                        .json(WsError { error: message })
                                                }
                                            }
                                        }
                                        HandlerCompilerResult::Error(message) => {
                                            HttpResponse::InternalServerError()
                                                .json(WsError { error: message })
                                        }
                                    }
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
