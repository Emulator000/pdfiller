use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, SeekFrom};
use std::io::{Seek, Write};
use std::str;

use serde_json::Value;

use pdf_forms::{FieldState, Form, LoadError, ValueError};

use lopdf::{Document as PdfDocument, Object, ObjectId};

use bytes::Buf;

use zip::write::FileOptions;

use crate::logger::Logger;
use crate::redis::models::document::Document;
use crate::utils::{get_filename, PATH_COMPILED};
use crate::utils::{read_file_buf, CustomDocumentRenumber};

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

struct PdfDocumentContainer {
    root: (ObjectId, Object),
    pages: BTreeMap<usize, (ObjectId, Object)>,
    objects: BTreeMap<ObjectId, Object>,
}

pub fn merge_compiled_documents(documents: &Vec<Document>) -> ExportCompilerResult {
    let documents_objects = get_documents_containers(documents);
    if documents_objects.is_empty() {
        ExportCompilerResult::Error("Cannot extract PDFs documents".into())
    } else {
        if let Some(mut document) = process_documents(documents_objects) {
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

fn get_documents_containers(documents: &Vec<Document>) -> Vec<PdfDocumentContainer> {
    let mut max_id = 1;

    let mut documents_objects = Vec::new();
    for (index, document) in documents.iter().enumerate() {
        if let Some(ref file_name) = get_compiled_filepath(&document.file) {
            match PdfDocument::load(file_name) {
                Ok(mut document) => {
                    document.renumber_objects_from_id(max_id);

                    max_id = document.max_id + 1;

                    let mut pages = document
                        .objects
                        .iter()
                        .filter(|(_, object)| object.type_name().unwrap_or("") == "Pages")
                        .map(|(object_id, object)| (*object_id, object.clone()))
                        .collect::<Vec<_>>();

                    if let Some(pages) = pages.pop() {
                        documents_objects.push(PdfDocumentContainer {
                            root: pages,
                            pages: document
                                .get_pages()
                                .into_iter()
                                .map(|(i, object_id)| {
                                    (
                                        index + i as usize,
                                        (
                                            object_id,
                                            document.get_object(object_id).unwrap().clone(),
                                        ),
                                    )
                                })
                                .collect(),
                            objects: document
                                .objects
                                .iter()
                                .filter(|(_, object)| {
                                    !vec!["Page", "Pages"]
                                        .contains(&object.type_name().unwrap_or(""))
                                })
                                .map(|(object_id, object)| (*object_id, object.clone()))
                                .collect(),
                        });
                    }
                }
                Err(e) => {
                    sentry::capture_error(&e);

                    Logger::log(format!("Error loading the PDF: {:#?}", e));
                }
            }
        }
    }

    documents_objects
}

fn process_documents(documents_objects: Vec<PdfDocumentContainer>) -> Option<PdfDocument> {
    let mut document = PdfDocument::new();
    document.version = PDF_VERSION.into();

    let mut catalog_object = None;
    let mut pages_object = None;

    let mut pages: Vec<ObjectId> = Vec::new();
    for document_objects in documents_objects {
        if pages_object.is_none() {
            pages_object = Some(document_objects.root);
        }

        let pages_object = pages_object.as_ref().unwrap();

        for (object_id, object) in document_objects.objects.iter() {
            match object.type_name().unwrap_or("") {
                "Catalog" => {
                    if catalog_object.is_none() {
                        catalog_object = Some((*object_id, object.clone()));
                    }
                }
                _ => {
                    document.objects.insert(*object_id, object.clone());
                }
            }
        }

        for (_, (object_id, object)) in document_objects.pages.iter() {
            if let Ok(dictionary) = object.as_dict() {
                let mut dictionary = dictionary.clone();
                dictionary.set("Parent", pages_object.0);

                document
                    .objects
                    .insert(*object_id, Object::Dictionary(dictionary));

                pages.push(*object_id);
            }
        }
    }

    if catalog_object.is_none() || pages_object.is_none() {
        return None;
    }

    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();

    if let Ok(dictionary) = pages_object.1.as_dict() {
        let count = pages.len();

        let mut dictionary = dictionary.clone();
        dictionary.set(
            "Kids",
            pages
                .into_iter()
                .map(|id| id.into())
                .collect::<Vec<Object>>(),
        );
        dictionary.set("Count", count as u32);

        document
            .objects
            .insert(pages_object.0, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);

        document
            .objects
            .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    document.compress();

    Some(document)
}

pub fn get_compiled_filepath<S: AsRef<str>>(filename: S) -> Option<String> {
    match get_filename(filename) {
        Some(file_name) => Some(format!("{}{}", PATH_COMPILED, file_name)),
        None => None,
    }
}
