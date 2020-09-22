use actix_web::{get, post, web, HttpResponse, Responder};

use crate::data::Data;
use crate::redis::models::document::Document;
use crate::services::WsError;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_documents);
    cfg.service(get_documents_by_token);
    cfg.service(get_document);
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
    if let Some(reviews) = data
        .redis
        .get_all_by::<Document, _>("token", &token.0)
        .await
    {
        HttpResponse::Ok().json(reviews)
    } else {
        HttpResponse::NoContent().json(WsError {
            error: "No reviews found!".into(),
        })
    }
}

#[get("/document/{document_id}")]
pub async fn get_document(data: web::Data<Data>, document_id: web::Path<i64>) -> impl Responder {
    if let Some(document) = data.redis.get::<Document>(document_id.0).await {
        HttpResponse::Ok().json(document.as_ref())
    } else {
        HttpResponse::NotFound().json(WsError {
            error: format!("Document {} not found!", document_id.0),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentRequest {
    file: String,
    token: String,
}

#[post("/document")]
pub async fn post_document(
    document_request: web::Json<DocumentRequest>,
    data: web::Data<Data>,
) -> impl Responder {
    match data
        .redis
        .create::<Document>(Document {
            id: 0,
            token: document_request.token.clone(),
            file: "".to_string(),
        })
        .await
    {
        Ok(_) => {
            if let Some(document) = data
                .redis
                .get_by::<Document, _>("token", &document_request.token)
                .await
            {
                HttpResponse::Created().json(document.as_ref())
            } else {
                HttpResponse::NotAcceptable().json(WsError {
                    error: format!("Document {} not created!", document_request.file),
                })
            }
        }
        Err(e) => HttpResponse::InternalServerError().json(WsError {
            error: format!("An error occurred: {:#?}", e),
        }),
    }
}
