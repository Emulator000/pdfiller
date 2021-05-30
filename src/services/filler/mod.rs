pub mod compiler;
mod form;
mod processor;

use std::str;

use actix_web::{post, web, HttpResponse, Responder};

use serde::Deserialize;
use serde_json::Value;

use crate::data::Data;
use crate::services::{self, WsError};

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
    request: web::HttpRequest,
    bytes: web::Bytes,
) -> impl Responder {
    match str::from_utf8(&bytes) {
        Ok(body) => match serde_json::from_str::<Value>(body) {
            Ok(values) => {
                if let Some(value) = values.get("data") {
                    match <compiler::PDFillerMap>::deserialize(value) {
                        Ok(ref map) => {
                            if let Some(documents) =
                                data.get_documents_by_token(token.as_str()).await
                            {
                                if documents.is_empty() {
                                    return HttpResponse::NotFound().json(WsError {
                                        error: "No documents found for this token!".into(),
                                    });
                                }

                                match compiler::compile_documents(
                                    data.file.clone(),
                                    map,
                                    &documents,
                                )
                                .await
                                {
                                    Ok(_) => {
                                        if let Some(accept) =
                                            services::get_accepted_header(&request)
                                        {
                                            let export_result =
                                                if accept.as_str() == mime::APPLICATION_PDF {
                                                    compiler::merge_documents(
                                                        data.file.clone(),
                                                        documents,
                                                        true,
                                                    )
                                                    .await
                                                } else {
                                                    compiler::zip_documents(
                                                        data.file.clone(),
                                                        documents,
                                                        true,
                                                    )
                                                    .await
                                                };

                                            services::export_content(accept, export_result)
                                        } else {
                                            HttpResponse::NotAcceptable().json(WsError {
                                                error: "Only PDF or Streams are accepted".into(),
                                            })
                                        }
                                    }
                                    Err(compiler::HandlerCompilerError::FillingError(e)) => {
                                        HttpResponse::BadRequest().json(WsError {
                                            error: format!(
                                                "Error during document filling: {:#?}",
                                                e
                                            ),
                                        })
                                    }
                                    Err(compiler::HandlerCompilerError::Error(message)) => {
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
