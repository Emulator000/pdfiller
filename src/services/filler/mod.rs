pub mod compiler;
mod form;
mod processor;

use std::str;

use actix_web::dev::BodyEncoding;
use actix_web::http::ContentEncoding;
use actix_web::{post, web, HttpResponse, Responder};

use serde::Deserialize;
use serde_json::Value;

use crate::data::Data;
use crate::redis::models::document::Document;
use crate::services::WsError;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(compile_documents);
}

#[derive(Deserialize)]
pub struct CompileOptions {
    pub merge: Option<bool>,
}

#[post("/compile/{token}")]
pub async fn compile_documents(
    data: web::Data<Data>,
    token: web::Path<String>,
    parameters: web::Query<CompileOptions>,
    bytes: web::Bytes,
) -> impl Responder {
    match str::from_utf8(&bytes) {
        Ok(body) => match serde_json::from_str::<Value>(body) {
            Ok(values) => {
                if let Some(value) = values.get("data") {
                    match <compiler::PDFillerMap>::deserialize(value) {
                        Ok(ref map) => {
                            if let Some(documents) =
                                data.redis.get_all_by::<Document, _>(&token.0).await
                            {
                                if documents.is_empty() {
                                    return HttpResponse::NotFound().json(WsError {
                                        error: "No documents found for this token!".into(),
                                    });
                                }

                                match compiler::compile_documents(map, &documents).await {
                                    compiler::HandlerCompilerResult::Success => {
                                        let merge = parameters.merge.unwrap_or(false);
                                        let export_result = if merge {
                                            compiler::merge_compiled_documents(documents)
                                        } else {
                                            compiler::zip_compiled_documents(documents)
                                        };

                                        match export_result {
                                            compiler::ExportCompilerResult::Success(bytes) => {
                                                HttpResponse::Ok()
                                                    .encoding(ContentEncoding::Identity)
                                                    .content_type(format!(
                                                        "{}",
                                                        if merge {
                                                            "application/pdf"
                                                        } else {
                                                            "application/zip"
                                                        }
                                                    ))
                                                    .header("accept-ranges", "bytes")
                                                    .header(
                                                        "content-disposition",
                                                        format!(
                                                            "attachment; filename=\"pdfs.{}\"",
                                                            if merge { "pdf" } else { "zip" }
                                                        ),
                                                    )
                                                    .body(bytes)
                                            }
                                            compiler::ExportCompilerResult::Error(message) => {
                                                HttpResponse::InternalServerError()
                                                    .json(WsError { error: message })
                                            }
                                        }
                                    }
                                    compiler::HandlerCompilerResult::FillingError(e) => {
                                        HttpResponse::BadRequest().json(WsError {
                                            error: format!(
                                                "Error during document filling: {:#?}",
                                                e
                                            ),
                                        })
                                    }
                                    compiler::HandlerCompilerResult::Error(message) => {
                                        HttpResponse::InternalServerError()
                                            .json(WsError { error: message })
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
