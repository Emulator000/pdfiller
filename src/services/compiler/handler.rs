use std::collections::HashMap;
use std::fs;
use std::path::Path;

use actix_web::HttpResponse;

use serde_json::Value;

use pdf_form_ids::{FieldType, Form};

use crate::redis::models::document::Document;
use crate::services::WsError;
use crate::utils::PATH_COMPILED;

pub type PDFillerMap = HashMap<String, Value>;

const REQUIRED_MARKER: char = '!';

pub fn compile_documents(map: &PDFillerMap, documents: &Vec<Document>) -> HttpResponse {
    for document in documents.iter() {
        match Form::load(&document.file) {
            Ok(mut form) => {
                fields_handler(map, &mut form);

                match Path::new(&document.file).file_name() {
                    Some(file_name) => {
                        if let Some(file_name) = file_name.to_str() {
                            match fs::create_dir_all(PATH_COMPILED) {
                                Ok(_) => {
                                    match form.save(&format!("{}{}", PATH_COMPILED, file_name)) {
                                        Ok(_) => {}
                                        Err(e) => {
                                            sentry::capture_error(&e);

                                            return HttpResponse::InternalServerError().json(
                                                WsError {
                                                    error: format!(
                                                        "Error {:#?} saving a PDF file, aborted.",
                                                        e
                                                    ),
                                                },
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    sentry::capture_error(&e);

                                    return HttpResponse::InternalServerError().json(WsError {
                                        error: format!(
                                            "Error {:#?} saving a PDF file, aborted.",
                                            e
                                        ),
                                    });
                                }
                            }
                        } else {
                            return HttpResponse::InternalServerError().json(WsError {
                                error: "Error saving a PDF file, aborted.".into(),
                            });
                        }
                    }
                    None => {
                        return HttpResponse::InternalServerError().json(WsError {
                            error: "Error saving a PDF file, aborted.".into(),
                        });
                    }
                }
            }
            Err(e) => {
                return HttpResponse::InternalServerError().json(WsError {
                    error: format!("Error {:#?} fetching a PDF file, aborted.", e),
                });
            }
        }
    }

    HttpResponse::Ok().body("PDF processed successfully.")
}

fn fields_handler(map: &PDFillerMap, form: &mut Form) {
    for (index, name) in form.get_all_names().iter().enumerate() {
        if let Some(name) = name {
            if let Some(value) = map.get(name.trim_start_matches(REQUIRED_MARKER)) {
                match form.get_type(index) {
                    FieldType::Text => {
                        let _ = form.set_text(index, value.as_str().unwrap_or("").into());
                    }
                    FieldType::Button => {}
                    FieldType::Radio => {
                        let _ = form.set_radio(index, value.as_str().unwrap_or("").into());
                    }
                    FieldType::CheckBox => {
                        let _ = form.set_check_box(index, value.as_bool().unwrap_or(false));
                    }
                    FieldType::ListBox => match value.as_array() {
                        Some(values) => {
                            let _ = form.set_list_box(
                                index,
                                values
                                    .iter()
                                    .map(|value| value.as_str().unwrap_or("").to_string())
                                    .collect(),
                            );
                        }
                        None => {}
                    },
                    FieldType::ComboBox => match value.as_array() {
                        Some(values) => {
                            let _ = form.set_combo_box(
                                index,
                                values
                                    .iter()
                                    .map(|value| value.as_str().unwrap_or("").to_string())
                                    .collect(),
                            );
                        }
                        None => {}
                    },
                }
            }
        }
    }
}
