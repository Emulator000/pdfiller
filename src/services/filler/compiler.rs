use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, SeekFrom};
use std::io::{Seek, Write};
use std::str;

use serde_json::Value;

use pdf_forms::LoadError;

use lopdf::Error;

use bytes::Buf;

use zip::write::FileOptions;

use uuid::Uuid;

use crate::redis::models::document::Document;
use crate::services::filler::form;
use crate::services::filler::form::FillingError;
use crate::services::filler::processor;
use crate::utils;

pub type PDFillerMap = HashMap<String, Value>;

pub const PATH: &'static str = "./tmp/";
pub const PATH_COMPILED: &'static str = "./tmp/compiled/";

pub enum HandlerCompilerResult {
    Success,
    FillingError(FillingError),
    Error(String),
}

pub enum ExportCompilerResult {
    Success(Vec<u8>),
    Error(String),
}

pub fn file_path<S: AsRef<str>>(filename: S) -> String {
    format!(
        "{}{}{}",
        PATH,
        Uuid::new_v4().to_string(),
        sanitize_filename::sanitize(filename.as_ref())
    )
}

pub async fn compile_documents(
    map: &PDFillerMap,
    documents: &Vec<Document>,
) -> HandlerCompilerResult {
    for document in documents.iter() {
        match compile_document(map, &document).await {
            HandlerCompilerResult::FillingError(e) => {
                return HandlerCompilerResult::FillingError(e);
            }
            HandlerCompilerResult::Error(message) => {
                return HandlerCompilerResult::Error(message);
            }
            HandlerCompilerResult::Success => {}
        }
    }

    HandlerCompilerResult::Success
}

pub async fn compile_document(map: &PDFillerMap, document: &Document) -> HandlerCompilerResult {
    match form::fields_filler(map, document).await {
        Ok(mut form) => {
            if let Some(compiled_filename) = get_compiled_filepath(&document.file) {
                match fs::create_dir_all(PATH_COMPILED) {
                    Ok(_) => match form.save(&compiled_filename) {
                        Ok(_) => HandlerCompilerResult::Success,
                        Err(e) => {
                            sentry::capture_error(&e);

                            HandlerCompilerResult::Error(format!(
                                "Error {:#?} saving a PDF file, aborted.",
                                e
                            ))
                        }
                    },
                    Err(e) => {
                        sentry::capture_error(&e);

                        HandlerCompilerResult::Error(format!(
                            "Error {:#?} saving a PDF file, aborted.",
                            e
                        ))
                    }
                }
            } else {
                HandlerCompilerResult::Error("Error saving a PDF file, aborted.".into())
            }
        }
        Err(e) => match e {
            FillingError::Load(e) => match e {
                LoadError::LopdfError(e) => match e {
                    Error::DictKey => {
                        if let Some(compiled_filename) = get_compiled_filepath(&document.file) {
                            match fs::create_dir_all(PATH_COMPILED) {
                                Ok(_) => {
                                    let _ = std::fs::copy(&document.file, &compiled_filename);

                                    HandlerCompilerResult::Success
                                }
                                Err(e) => {
                                    sentry::capture_error(&e);

                                    HandlerCompilerResult::Error(format!(
                                        "Error {:#?} saving a PDF file, aborted.",
                                        e
                                    ))
                                }
                            }
                        } else {
                            HandlerCompilerResult::Error(format!(
                                "Error saving a PDF file, aborted.",
                            ))
                        }
                    }
                    _ => {
                        HandlerCompilerResult::Error(format!("Error saving a PDF file, aborted.",))
                    }
                },
                _ => HandlerCompilerResult::Error(format!("Error saving a PDF file, aborted.",)),
            },
            _ => HandlerCompilerResult::FillingError(e),
        },
    }
}

pub fn zip_compiled_documents(documents: &Vec<Document>) -> ExportCompilerResult {
    let buf = Vec::new();
    let w = std::io::Cursor::new(buf);
    let mut zip = zip::ZipWriter::new(w);

    for document in documents.iter() {
        if let Some(ref file_name) = utils::get_filename(&document.file) {
            match zip.start_file(file_name, FileOptions::default()) {
                Ok(_) => match get_compiled_filepath(&document.file) {
                    Some(ref compiled_file_name) => {
                        match utils::read_file_buf(compiled_file_name) {
                            Some(buffer) => match zip.write(&buffer) {
                                Ok(_) => {}
                                Err(e) => {
                                    return ExportCompilerResult::Error(format!(
                                        "Error making a ZIP file: {:#?}",
                                        e
                                    ));
                                }
                            },
                            None => {
                                return ExportCompilerResult::Error(
                                    "Error making a ZIP file.".into(),
                                );
                            }
                        }
                    }
                    None => {
                        return ExportCompilerResult::Error("Error making a ZIP file.".into());
                    }
                },
                Err(e) => {
                    return ExportCompilerResult::Error(format!(
                        "Error making a ZIP file: {:#?}",
                        e
                    ));
                }
            }
        }
    }

    let zip_result = zip.finish();
    let mut bytes = zip_result.unwrap_or_default();
    let _ = bytes.seek(SeekFrom::Start(0));

    ExportCompilerResult::Success(bytes.bytes().to_vec())
}

pub fn merge_compiled_documents(documents: &Vec<Document>) -> ExportCompilerResult {
    let documents_objects = processor::get_documents_containers(documents);
    if documents_objects.pages.is_empty() || documents_objects.objects.is_empty() {
        ExportCompilerResult::Error("Cannot extract PDFs documents".into())
    } else {
        if let Some(mut document) = processor::process_documents(documents_objects) {
            let buf = Vec::<u8>::new();
            let mut cursor = Cursor::new(buf);

            match document.save_to(&mut cursor) {
                Ok(_) => {
                    let _ = cursor.seek(SeekFrom::Start(0));

                    ExportCompilerResult::Success(cursor.bytes().to_vec())
                }
                Err(e) => ExportCompilerResult::Error(format!(
                    "An error {:#?} occurred saving the PDFs files.",
                    e
                )),
            }
        } else {
            ExportCompilerResult::Error(format!("Error decoding the PDFs files."))
        }
    }
}

pub fn get_compiled_filepath<S: AsRef<str>>(filename: S) -> Option<String> {
    match utils::get_filename(filename) {
        Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
        None => None,
    }
}
