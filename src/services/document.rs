use actix_multipart::{Field, Multipart, MultipartError};
use actix_web::{get, post, web, HttpResponse, Responder};

use futures_lite::stream::StreamExt;

use crate::data::{Data, DataResult};
use crate::file::FileResult;
use crate::mongo::models::document::Document;
use crate::services::{self, filler::compiler, WsError};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(post_document);
    cfg.service(get_document);
    cfg.service(get_documents);
    cfg.service(get_documents_by_token);
}

#[derive(Debug, Deserialize)]
pub struct FormData {
    file: String,
}

#[post("/document/{token}")]
pub async fn post_document(
    data: web::Data<Data>,
    token: web::Path<String>,
    request: web::HttpRequest,
    form: Option<web::Form<FormData>>,
    mut payload: Multipart,
) -> impl Responder {
    if let Some(accept) = services::get_accepted_header(&request) {
        if accept.as_str() == mime::APPLICATION_PDF {
            let mut filepath = None;
            if let Some(form) = form {
                filepath = data.file.download_and_save(form.file.as_str()).await;
            } else {
                while let Ok(Some(mut field)) = payload.try_next().await {
                    match field.content_disposition() {
                        Some(ref content_type) => match content_type.get_name() {
                            Some("file") => match content_type.get_filename() {
                                Some(filename) => {
                                    if !filename.is_empty() {
                                        match read_chuncked_buffer(&mut field).await {
                                            Ok(buf) => {
                                                let local_filepath =
                                                    data.file.generate_filepath(&filename);
                                                match data.file.save(&local_filepath, buf).await {
                                                    FileResult::Saved => {
                                                        filepath = Some(local_filepath);
                                                    }
                                                    FileResult::S3Error(e) => {
                                                        sentry::capture_error(&e);

                                                        return HttpResponse::InternalServerError().json(WsError {
                                                            error: format!(
                                                                "An error occurred uploading the file: {:#?}",
                                                                e
                                                            ),
                                                        });
                                                    }
                                                    FileResult::Error(e) => {
                                                        sentry::capture_error(&e);

                                                        return HttpResponse::InternalServerError().json(WsError {
                                                            error: format!(
                                                                "An error occurred uploading the file: {:#?}",
                                                                e
                                                            ),
                                                        });
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            Err(e) => {
                                                sentry::capture_error(&e);

                                                return HttpResponse::InternalServerError().json(WsError {
                                                    error: format!(
                                                        "An error occurred uploading the file: {:#?}",
                                                        e
                                                    ),
                                                });
                                            }
                                        }
                                    }
                                }
                                None => match read_chuncked_buffer(&mut field).await {
                                    Ok(buf) => match std::str::from_utf8(buf.as_slice()) {
                                        Ok(uri) => {
                                            filepath = data.file.download_and_save(uri).await;
                                        }
                                        Err(e) => {
                                            sentry::capture_error(&e);

                                            return HttpResponse::InternalServerError().json(WsError {
                                                error: format!(
                                                    "An error occurred downloading the remote file: {:#?}",
                                                    e
                                                ),
                                            });
                                        }
                                    },
                                    Err(e) => {
                                        sentry::capture_error(&e);

                                        return HttpResponse::InternalServerError().json(WsError {
                                            error: format!(
                                                "An error occurred uploading the file: {:#?}",
                                                e
                                            ),
                                        });
                                    }
                                },
                            },
                            Some(_) => {}
                            None => {}
                        },
                        None => {}
                    }
                }
            }

            if filepath.is_none() {
                HttpResponse::BadRequest().json(WsError {
                    error: "File missing.".into(),
                })
            } else {
                if let Some(file) = filepath {
                    let document = Document::new(token.0, file);
                    match data.create_document(document.clone()).await {
                        DataResult::Ok => HttpResponse::Created().json(document),
                        DataResult::Error(e) => HttpResponse::InternalServerError().json(WsError {
                            error: format!("An error occurred: {:#?}", e),
                        }),
                    }
                } else {
                    HttpResponse::InternalServerError().json(WsError {
                        error: "An error occurred".into(),
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
    if let Some(documents) = data.get_documents_by_token(&token.0).await {
        if documents.is_empty() {
            return HttpResponse::NotFound().json(WsError {
                error: "No documents found for this token!".into(),
            });
        }

        if let Some(accept) = services::get_accepted_header(&request) {
            let export_result = if accept.as_str() == mime::APPLICATION_PDF {
                compiler::merge_documents(data.file.clone(), documents, false).await
            } else {
                compiler::zip_documents(data.file.clone(), documents, false).await
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
    if let Some(documents) = data.get_all_documents().await {
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
    if let Some(documents) = data.get_documents_by_token(&token.0).await {
        HttpResponse::Ok().json(documents)
    } else {
        HttpResponse::NoContent().json(WsError {
            error: "No documents found!".into(),
        })
    }
}

async fn read_chuncked_buffer(field: &mut Field) -> Result<Vec<u8>, MultipartError> {
    let mut buf = Vec::new();
    while let Some(chunk) = field.next().await {
        match chunk {
            Ok(data) => {
                buf.extend(data);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(buf)
}
