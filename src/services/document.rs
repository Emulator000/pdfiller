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
    cfg.service(get_documents);
    cfg.service(get_documents_by_token);
    cfg.service(post_document);
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
    if let Some(reviews) = data.redis.get_all_by::<Document, _>(&token.0).await {
        HttpResponse::Ok().json(reviews)
    } else {
        HttpResponse::NoContent().json(WsError {
            error: "No documents found!".into(),
        })
    }
}

#[post("/document")]
pub async fn post_document(mut payload: Multipart, data: web::Data<Data>) -> impl Responder {
    let mut token = None;
    let mut filepath = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        match field.content_disposition() {
            Some(ref content_type) => match content_type.get_name() {
                Some("file") => match content_type.get_filename() {
                    Some(filename) => match fs::create_dir_all(PATH) {
                        Ok(_) => {
                            let file = file_path(&filename);
                            match web::block(|| fs::File::create(file)).await {
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
                                            }
                                        }
                                    }

                                    filepath = Some(file_path(&filename));
                                }
                                Err(e) => {
                                    sentry::capture_error(&e);
                                }
                            }
                        }
                        Err(e) => {
                            sentry::capture_error(&e);
                        }
                    },
                    None => {}
                },
                Some("token") => {
                    token = Some(read_chunked_string(&mut field).await);
                }
                Some(_) => {}
                None => {}
            },
            None => {}
        }
    }

    if token.is_none() || filepath.is_none() {
        HttpResponse::BadRequest().json(WsError {
            error: format!("A field is missing."),
        })
    } else {
        if let Some(token) = token {
            if let Some(file) = filepath {
                let document = Document {
                    token: token.clone(),
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
        } else {
            HttpResponse::InternalServerError().json(WsError {
                error: format!("An error occurred"),
            })
        }
    }
}
