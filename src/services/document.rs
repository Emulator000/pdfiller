use std::fs;
use std::io::Write;

use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpResponse, Responder};

use futures_lite::stream::StreamExt;

use crate::data::Data;
use crate::redis::models::document::Document;
use crate::services::{self, filler::compiler, WsError};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(post_document);
    cfg.service(get_document);
    cfg.service(get_documents);
    cfg.service(get_documents_by_token);
}

#[post("/document/{token}")]
pub async fn post_document(
    data: web::Data<Data>,
    token: web::Path<String>,
    request: web::HttpRequest,
    mut payload: Multipart,
) -> impl Responder {
    if let Some(accept) = services::get_accepted_header(&request) {
        if accept.as_str() == mime::APPLICATION_PDF {
            let mut filepath = None;

            while let Ok(Some(mut field)) = payload.try_next().await {
                match field.content_disposition() {
                    Some(ref content_type) => match content_type.get_name() {
                        Some("file") => match content_type.get_filename() {
                            Some(filename) => match fs::create_dir_all(compiler::PATH) {
                                Ok(_) => {
                                    let local_filepath = compiler::file_path(&filename);
                                    filepath = Some(local_filepath.clone());

                                    match web::block(|| fs::File::create(local_filepath)).await {
                                        Ok(mut file) => {
                                            while let Some(chunk) = field.next().await {
                                                match chunk {
                                                    Ok(data) => match web::block(move || {
                                                        file.write_all(&data).map(|_| file)
                                                    })
                                                    .await
                                                    {
                                                        Ok(f) => {
                                                            file = f;
                                                        }
                                                        Err(e) => {
                                                            sentry::capture_error(&e);

                                                            return HttpResponse::InternalServerError().json(
                                                                WsError {
                                                                    error: format!("An error occurred during upload: {:#?}", e),
                                                                },
                                                            );
                                                        }
                                                    },
                                                    Err(e) => {
                                                        sentry::capture_error(&e);

                                                        filepath = None;
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            sentry::capture_error(&e);

                                            filepath = None;
                                        }
                                    }
                                }
                                Err(e) => {
                                    sentry::capture_error(&e);
                                }
                            },
                            None => {}
                        },
                        Some(_) => {}
                        None => {}
                    },
                    None => {}
                }
            }

            if filepath.is_none() {
                HttpResponse::BadRequest().json(WsError {
                    error: format!("File missing."),
                })
            } else {
                if let Some(file) = filepath {
                    let document = Document {
                        token: token.0,
                        file,
                    };

                    match data.redis.create::<Document>(document.clone()).await {
                        Ok(_) => HttpResponse::Created().json(document),
                        Err(e) => HttpResponse::InternalServerError().json(WsError {
                            error: format!("An error occurred: {:#?}", e),
                        }),
                    }
                } else {
                    HttpResponse::InternalServerError().json(WsError {
                        error: format!("An error occurred"),
                    })
                }
            }
        } else {
            HttpResponse::NotAcceptable().json(WsError {
                error: "Only PDF are accepted".into(),
            })
        }
    } else {
        HttpResponse::NotAcceptable().json(WsError {
            error: "Only PDF are accepted".into(),
        })
    }
}

#[get("/document/{token}")]
pub async fn get_document(
    data: web::Data<Data>,
    token: web::Path<String>,
    request: web::HttpRequest,
) -> impl Responder {
    if let Some(documents) = data.redis.get_all_by::<Document, _>(&token.0).await {
        if documents.is_empty() {
            return HttpResponse::NotFound().json(WsError {
                error: "No documents found for this token!".into(),
            });
        }

        if let Some(accept) = services::get_accepted_header(&request) {
            let export_result = if accept.as_str() == mime::APPLICATION_PDF {
                compiler::merge_documents(documents, false)
            } else {
                compiler::zip_documents(documents, false)
            };

            services::export_content(accept, export_result)
        } else {
            HttpResponse::NotAcceptable().json(WsError {
                error: "Only PDF or Streams are accepted".into(),
            })
        }
    } else {
        HttpResponse::NotFound().json(WsError {
            error: "No documents found for this token!".into(),
        })
    }
}

#[get("/documents")]
pub async fn get_documents(data: web::Data<Data>) -> impl Responder {
    if let Some(documents) = data.redis.get_all::<Document>().await {
        HttpResponse::Ok().json(documents)
    } else {
        HttpResponse::NoContent().json(WsError {
            error: "No documents found!".into(),
        })
    }
}

#[get("/documents/{token}")]
pub async fn get_documents_by_token(
    data: web::Data<Data>,
    token: web::Path<String>,
) -> impl Responder {
    if let Some(documents) = data.redis.get_all_by::<Document, _>(&token.0).await {
        HttpResponse::Ok().json(documents)
    } else {
        HttpResponse::NoContent().json(WsError {
            error: "No documents found!".into(),
        })
    }
}
