mod document;
mod filler;

use actix_web::dev::BodyEncoding;
use actix_web::http::ContentEncoding;
use actix_web::{web, HttpResponse};

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

pub fn export_content<S: AsRef<str>>(
    accept: S,
    export_result: compiler::ExportCompilerResult,
) -> HttpResponse {
    match export_result {
        compiler::ExportCompilerResult::Success(bytes) => HttpResponse::Ok()
            .encoding(ContentEncoding::Identity)
            .content_type(accept.as_ref())
            .header("accept-ranges", "bytes")
            .header(
                "content-disposition",
                format!(
                    "attachment; filename=\"pdf.{}\"",
                    if accept.as_ref() != mime::APPLICATION_PDF {
                        "zip"
                    } else {
                        "pdf"
                    }
                ),
            )
            .body(bytes),
        compiler::ExportCompilerResult::Error(message) => {
            HttpResponse::InternalServerError().json(WsError { error: message })
        }
    }
}
