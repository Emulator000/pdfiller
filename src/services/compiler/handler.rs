use std::collections::HashMap;
use std::fs;
use std::io::SeekFrom;
use std::io::{Seek, Write};
use std::str;

use serde_json::Value;

use pdf_form_ids::{FieldType, Form};

use bytes::Buf;

use zip::write::FileOptions;

use crate::redis::models::document::Document;
use crate::utils::read_file_buf;
use crate::utils::{get_filename, PATH_COMPILED};

pub type PDFillerMap = HashMap<String, Value>;

const REQUIRED_MARKER: char = '!';

pub enum HandlerCompilerResult {
    Success,
    Error(String),
}

pub enum ZipCompilerResult {
    Success(Vec<u8>),
    Error(String),
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

pub fn compile_documents(map: &PDFillerMap, documents: &Vec<Document>) -> HandlerCompilerResult {
    for document in documents.iter() {
        if let HandlerCompilerResult::Error(message) = compile_document(map, &document) {
            return HandlerCompilerResult::Error(message);
        }
    }

    HandlerCompilerResult::Success
}

pub fn compile_document(map: &PDFillerMap, document: &Document) -> HandlerCompilerResult {
    match Form::load(&document.file) {
        Ok(mut form) => {
            fields_handler(map, &mut form);

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
        Err(e) => {
            HandlerCompilerResult::Error(format!("Error {:#?} fetching a PDF file, aborted.", e))
        }
    }
}

pub fn zip_compiled_documents(documents: &Vec<Document>) -> ZipCompilerResult {
    let buf = Vec::new();
    let w = std::io::Cursor::new(buf);
    let mut zip = zip::ZipWriter::new(w);

    for document in documents.iter() {
        if let Some(ref file_name) = get_filename(&document.file) {
            match zip.start_file(file_name, FileOptions::default()) {
                Ok(_) => match get_compiled_filepath(&document.file) {
                    Some(ref compiled_file_name) => match read_file_buf(compiled_file_name) {
                        Some(buffer) => match zip.write(&buffer) {
                            Ok(_) => {}
                            Err(e) => {
                                return ZipCompilerResult::Error(format!(
                                    "Error making a ZIP file: {:#?}",
                                    e
                                ));
                            }
                        },
                        None => {
                            return ZipCompilerResult::Error("Error making a ZIP file.".into());
                        }
                    },
                    None => {
                        return ZipCompilerResult::Error("Error making a ZIP file.".into());
                    }
                },
                Err(e) => {
                    return ZipCompilerResult::Error(format!("Error making a ZIP file: {:#?}", e));
                }
            }
        }
    }

    let zip_result = zip.finish();
    let mut bytes = zip_result.unwrap_or_default();
    let _ = bytes.seek(SeekFrom::Start(0));

    ZipCompilerResult::Success(bytes.bytes().to_vec())
}

pub fn get_compiled_filepath<S: AsRef<str>>(filename: S) -> Option<String> {
    match get_filename(filename) {
        Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
        None => None,
    }
}
