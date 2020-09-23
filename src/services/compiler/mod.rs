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
    get_compiled_filepath, zip_compiled_documents, ExportCompilerResult, HandlerCompilerResult,
};
use crate::services::WsError;
use crate::utils::read_file_buf;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(compile_documents);
}

#[post("/compile/{token}")]
pub async fn compile_documents(
    data: web::Data<Data>,
    token: web::Path<String>,
    bytes: web::Bytes,
) -> impl Responder {
    match str::from_utf8(&bytes) {
        Ok(body) => match serde_json::from_str::<Value>(body) {
            Ok(values) => {
                if let Some(value) = values.get("data") {
                    match <handler::PDFillerMap>::deserialize(value) {
                        Ok(ref map) => {
                            if let Some(mut documents) =
                                data.redis.get_all_by::<Document, _>(&token.0).await
                            {
                                if documents.is_empty() {
                                    return HttpResponse::NotFound().json(WsError {
                                        error: "No documents found for this token!".into(),
                                    });
                                }

                                if documents.len() == 1 {
                                    match documents.pop() {
                                        Some(document) => {
                                            match handler::compile_document(map, &document) {
                                                HandlerCompilerResult::Success => {
                                                    match get_compiled_filepath(&document.file) {
                                                        Some(file_path) => {
                                                            match read_file_buf(file_path) {
                                                                    Some(buffer) => HttpResponse::Ok()
                                                                        .encoding(ContentEncoding::Identity)
                                                                        .content_type("application/pdf")
                                                                        .header("accept-ranges", "bytes")
                                                                        .header(
                                                                            "content-disposition",
                                                                            "attachment; filename=\"compiled.pdf\"",
                                                                        )
                                                                        .body(buffer),
                                                                    None => HttpResponse::NotFound().json(WsError {
                                                                        error: "Error compiling the PDF".into(),
                                                                    })
                                                                }
                                                        }
                                                        None => {
                                                            HttpResponse::NotFound().json(WsError {
                                                                error: "Error compiling the PDF"
                                                                    .into(),
                                                            })
                                                        }
                                                    }
                                                }
                                                HandlerCompilerResult::Error(message) => {
                                                    HttpResponse::InternalServerError()
                                                        .json(WsError { error: message })
                                                }
                                            }
                                        }
                                        None => unreachable!(),
                                    }
                                } else {
                                    match handler::compile_documents(map, &documents) {
                                        HandlerCompilerResult::Success => {
                                            match zip_compiled_documents(&documents) {
                                                ExportCompilerResult::Success(bytes) => {
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
                                                ExportCompilerResult::Error(message) => {
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
