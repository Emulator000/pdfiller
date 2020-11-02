mod document;
mod filler;

use actix_web::dev::BodyEncoding;
use actix_web::http::ContentEncoding;
use actix_web::{body, web, HttpResponse};

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

pub fn export_content(
    multiple: bool,
    export_result: compiler::ExportCompilerResult,
) -> Response<body::Body> {
    match export_result {
        compiler::ExportCompilerResult::Success(bytes) => HttpResponse::Ok()
            .encoding(ContentEncoding::Identity)
            .content_type(&accept)
            .header("accept-ranges", "bytes")
            .header(
                "content-disposition",
                format!(
                    "attachment; filename=\"pdf.{}\"",
                    if multiple { "zip" } else { "pdf" }
                ),
            )
            .body(bytes),
        compiler::ExportCompilerResult::Error(message) => {
            HttpResponse::InternalServerError().json(WsError { error: message })
        }
    }
}
