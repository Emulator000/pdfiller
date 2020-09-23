use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, SeekFrom};
use std::io::{Seek, Write};
use std::str;

use serde_json::Value;

use pdf_forms::{FieldState, Form, LoadError, ValueError};

use lopdf::content::Content;
use lopdf::{Dictionary, Document as PdfDocument, Object, ObjectId, Stream};

use bytes::Buf;

use zip::write::FileOptions;

use crate::logger::Logger;
use crate::redis::models::document::Document;
use crate::utils::read_file_buf;
use crate::utils::{get_filename, PATH_COMPILED};

pub type PDFillerMap = HashMap<String, Value>;

const PDF_VERSION: &'static str = "1.4";
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
struct PdfPageContainer {
    contents: Vec<Content>,
    dictionary: Dictionary,
}

pub fn merge_compiled_documents(documents: &Vec<Document>) -> ExportCompilerResult {
    let mut objects = Vec::new();
    for document in documents.iter() {
        if let Some(ref file_name) = get_compiled_filepath(&document.file) {
            match PdfDocument::load(file_name) {
                Ok(document) => {
                    objects.append(&mut document.get_pages().iter().fold(
                        Vec::new(),
                        |mut pages, (_, page_id)| {
                            let mut contents = Vec::new();
                            if let Ok(content) = document.get_and_decode_page_content(*page_id) {
                                contents.push(content);
                            }

                            match document.get_dictionary(*page_id) {
                                Ok(dictionary) => {
                                    pages.push(PdfPageContainer {
                                        contents,
                                        dictionary: dictionary.to_owned(),
                                    });
                                }
                                Err(e) => {
                                    sentry::capture_error(&e);

                                    Logger::log(format!(
                                        "Error getting the PDF dictionary: {:#?}",
                                        e
                                    ));
                                }
                            }

                            pages
                        },
                    ));
                }
                Err(e) => {
                    sentry::capture_error(&e);

                    Logger::log(format!("Error loading the PDF: {:#?}", e));
                }
            }
        }
    }

    if objects.is_empty() {
        ExportCompilerResult::Error("Cannot extract PDFs pages".into())
    } else {
        let mut doc = PdfDocument::new();
        doc.version = PDF_VERSION.to_string();

        let pages_id = doc.new_object_id();

        let mut pages: Vec<ObjectId> = Vec::new();
        for mut pdf_page in objects {
            let mut contents_ids: Vec<ObjectId> = Vec::new();
            for content in pdf_page.contents {
                contents_ids
                    .push(doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap())));
            }

            pdf_page.dictionary.set("Parent", pages_id);
            pdf_page.dictionary.set(
                "Contents",
                contents_ids
                    .into_iter()
                    .map(|id| id.into())
                    .collect::<Vec<Object>>(),
            );

            pages.push(doc.add_object(pdf_page.dictionary));
        }

        let count = pages.len() as i32;
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => pages.into_iter().map(|id| id.into()).collect::<Vec<Object>>(),
            "Count" => count,
        };

        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });

        doc.trailer.set("Root", catalog_id);

        let buf = Vec::<u8>::new();
        let mut cursor = Cursor::new(buf);

        match doc.save_to(&mut cursor) {
            Ok(_) => {
                let _ = cursor.seek(SeekFrom::Start(0));

                ExportCompilerResult::Success(cursor.bytes().to_vec())
            }
            Err(e) => ExportCompilerResult::Error(format!(
                "An error {:#?} occurred saving the PDFs files.",
                e
            )),
        }
    }
}

pub fn get_compiled_filepath<S: AsRef<str>>(filename: S) -> Option<String> {
    match get_filename(filename) {
        Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
        None => None,
    }
}
