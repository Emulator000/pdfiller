use std::fs;
use std::io::Write;

use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpResponse, Responder};

use futures_lite::stream::StreamExt;

use crate::data::Data;
use crate::redis::models::document::Document;
use crate::services::WsError;
use crate::utils::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(post_document);
    cfg.service(get_documents);
    cfg.service(get_documents_by_token);
}

#[post("/document/{token}")]
pub async fn post_document(
    data: web::Data<Data>,
    token: web::Path<String>,
    mut payload: Multipart,
) -> impl Responder {
    let mut filepath = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        match field.content_disposition() {
            Some(ref content_type) => match content_type.get_name() {
                Some("file") => match content_type.get_filename() {
                    Some(filename) => match fs::create_dir_all(PATH) {
                        Ok(_) => {
                            let local_filepath = file_path(&filename);
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
