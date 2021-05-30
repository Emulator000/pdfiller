mod document;
mod filler;

use actix_web::dev::BodyEncoding;
use actix_web::http::{header::ACCEPT, ContentEncoding};
use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::services::filler::compiler;

#[derive(Serialize)]
struct WsMessage {
    message: String,
}

#[derive(Serialize)]
struct WsError {
    error: String,
}

pub fn config(cfg: &mut web::ServiceConfig) {
    document::config(cfg);
    filler::config(cfg);
}

pub fn get_accepted_header(request: &web::HttpRequest) -> Option<String> {
    if let Some(accept) = request.headers().get(ACCEPT) {
        let accept = accept.to_str().unwrap_or("").to_lowercase();

        if accept.as_str() == mime::APPLICATION_PDF
            || accept.as_str() == mime::APPLICATION_OCTET_STREAM
        {
            Some(accept)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn export_content<S: AsRef<str>>(
    accept: S,
    export_result: compiler::ExportCompilerResult<Vec<u8>>,
) -> HttpResponse {
    match export_result {
        Ok(bytes) => HttpResponse::Ok()
            .encoding(ContentEncoding::Identity)
            .content_type(accept.as_ref())
            .append_header(("accept-ranges", "bytes"))
            .append_header((
                "content-disposition",
                format!(
                    "attachment; filename=\"pdf.{}\"",
                    if accept.as_ref() != mime::APPLICATION_PDF {
                        "zip"
                    } else {
                        "pdf"
                    }
                ),
            ))
            .body(bytes),
        Err(compiler::ExportCompilerError::GenericError(message)) => {
            HttpResponse::InternalServerError().json(WsError { error: message })
        }
    }
}
