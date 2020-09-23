use std::collections::HashMap;
use std::fs;
use std::io::SeekFrom;
use std::io::{Seek, Write};
use std::str;

use serde_json::Value;

use pdf_forms::{FieldState, Form, LoadError, ValueError};

use bytes::Buf;

use zip::write::FileOptions;

use crate::redis::models::document::Document;
use crate::utils::read_file_buf;
use crate::utils::{get_filename, PATH_COMPILED};

pub type PDFillerMap = HashMap<String, Value>;

const REQUIRED_MARKER: char = '!';

pub enum HandlerCompilerResult {
    Success,
    FillingError(FillingError),
    Error(String),
}

pub enum ExportCompilerResult {
    Success(Vec<u8>),
    Error(String),
}

#[derive(Debug)]
pub enum FillingError {
    Load(LoadError),
    Value(ValueError),
    RequiredField(String),
}

fn fields_filler(map: &PDFillerMap, document: &Document) -> Result<Form, FillingError> {
    match Form::load(&document.file) {
        Ok(mut form) => {
            for (index, name) in form.get_all_names().iter().enumerate() {
                if let Some(name) = name {
                    let value = map.get(name.trim_start_matches(REQUIRED_MARKER));
                    let result = match form.get_state(index) {
                        FieldState::Text { required, .. } => {
                            if required && value.is_none() {
                                Err(FillingError::RequiredField(name.to_owned()))
                            } else if let Some(value) = value {
                                form.set_text(index, value.as_str().unwrap_or("").into())
                                    .map_err(|e| FillingError::Value(e))
                            } else {
                                Ok(())
                            }
                        }
                        FieldState::Radio { required, .. } => {
                            if required && value.is_none() {
                                Err(FillingError::RequiredField(name.to_owned()))
                            } else if let Some(value) = value {
                                form.set_radio(index, value.as_str().unwrap_or("").into())
                                    .map_err(|e| FillingError::Value(e))
                            } else {
                                Ok(())
                            }
                        }
                        FieldState::CheckBox { required, .. } => {
                            if required && value.is_none() {
                                Err(FillingError::RequiredField(name.to_owned()))
                            } else if let Some(value) = value {
                                form.set_check_box(index, value.as_bool().unwrap_or(false))
                                    .map_err(|e| FillingError::Value(e))
                            } else {
                                Ok(())
                            }
                        }
                        FieldState::ListBox { required, .. } => {
                            if required && value.is_none() {
                                Err(FillingError::RequiredField(name.to_owned()))
                            } else if let Some(value) = value {
                                match value.as_array() {
                                    Some(values) => form
                                        .set_list_box(
                                            index,
                                            values
                                                .iter()
                                                .map(|value| {
                                                    value.as_str().unwrap_or("").to_string()
                                                })
                                                .collect(),
                                        )
                                        .map_err(|e| FillingError::Value(e)),
                                    None => Ok(()),
                                }
                            } else {
                                Ok(())
                            }
                        }
                        FieldState::ComboBox { required, .. } => {
                            if required && value.is_none() {
                                Err(FillingError::RequiredField(name.to_owned()))
                            } else if let Some(value) = value {
                                match value.as_array() {
                                    Some(values) => form
                                        .set_combo_box(
                                            index,
                                            values
                                                .iter()
                                                .map(|value| {
                                                    value.as_str().unwrap_or("").to_string()
                                                })
                                                .collect(),
                                        )
                                        .map_err(|e| FillingError::Value(e)),
                                    None => Ok(()),
                                }
                            } else {
                                Ok(())
                            }
                        }
                        _ => Ok(()),
                    };

                    if let Err(e) = result {
                        return Err(e);
                    }
                }
            }

            Ok(form)
        }
        Err(e) => Err(FillingError::Load(e)),
    }
}

pub fn compile_documents(map: &PDFillerMap, documents: &Vec<Document>) -> HandlerCompilerResult {
    for document in documents.iter() {
        match compile_document(map, &document) {
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

pub fn compile_document(map: &PDFillerMap, document: &Document) -> HandlerCompilerResult {
    match fields_filler(map, document) {
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
        Err(e) => HandlerCompilerResult::FillingError(e),
    }
}

pub fn zip_compiled_documents(documents: &Vec<Document>) -> ExportCompilerResult {
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
                                return ExportCompilerResult::Error(format!(
                                    "Error making a ZIP file: {:#?}",
                                    e
                                ));
                            }
                        },
                        None => {
                            return ExportCompilerResult::Error("Error making a ZIP file.".into());
                        }
                    },
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

pub fn get_compiled_filepath<S: AsRef<str>>(filename: S) -> Option<String> {
    match get_filename(filename) {
        Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
        None => None,
    }
}
