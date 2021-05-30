use std::collections::HashMap;
use std::io::{Cursor, SeekFrom};
use std::io::{Seek, Write};

use async_std::sync::Arc;

use serde_json::Value;

use pdf_forms::LoadError;

use lopdf::{Document as PdfDocument, Error};

use zip::write::FileOptions;

use crate::file::{FileError, FileProvider, FileResult};
use crate::mongo::models::document::Document;
use crate::services::filler::form;
use crate::services::filler::form::FillingError;
use crate::services::filler::processor;

pub type PDFillerMap = HashMap<String, Value>;

pub enum HandlerCompilerResult {
    Success,
    FillingError(FillingError),
    Error(String),
}

pub enum ExportCompilerResult {
    Success(Vec<u8>),
    Error(String),
}

pub async fn compile_documents<F: FileProvider + ?Sized>(
    file_type: Arc<Box<F>>,
    map: &PDFillerMap,
    documents: &[Document],
) -> HandlerCompilerResult {
    for document in documents.iter() {
        match compile_document(file_type.clone(), map, &document).await {
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

pub async fn compile_document<F: FileProvider + ?Sized>(
    file_type: Arc<Box<F>>,
    map: &PDFillerMap,
    document: &Document,
) -> HandlerCompilerResult {
    match form::fields_filler(map, document).await {
        Ok(mut form) => {
            if let Some(compiled_filename) =
                file_type.generate_compiled_filepath(document.file.as_str())
            {
                let mut buf = Vec::new();
                match form.save_to(&mut buf) {
                    Ok(_) => save_compiled_file(file_type, compiled_filename, buf).await,
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
                LoadError::LopdfError(Error::DictKey) => {
                    if let Some(compiled_filename) =
                        file_type.generate_compiled_filepath(&document.file)
                    {
                        match crystalsoft_utils::read_file_buf(&document.file) {
                            Ok(buf) => save_compiled_file(file_type, compiled_filename, buf).await,
                            Err(e) => {
                                sentry::capture_error(&e);

                                HandlerCompilerResult::Error(format!(
                                    "Error {:#?} saving a PDF file, aborted.",
                                    e
                                ))
                            }
                        }
                    } else {
                        HandlerCompilerResult::Error(
                            "Error saving a PDF file, aborted.".to_string(),
                        )
                    }
                }
                _ => HandlerCompilerResult::Error("Error saving a PDF file, aborted.".to_string()),
            },
            _ => HandlerCompilerResult::FillingError(e),
        },
    }
}

pub async fn zip_documents<F: FileProvider + ?Sized>(
    file_type: Arc<Box<F>>,
    documents: Vec<Document>,
    compiled: bool,
) -> ExportCompilerResult {
    let buf = Vec::new();
    let w = std::io::Cursor::new(buf);
    let mut zip = zip::ZipWriter::new(w);

    for document in documents {
        if let Some(ref file_name) = crystalsoft_utils::get_filename(&document.file) {
            match zip.start_file(file_name, FileOptions::default()) {
                Ok(_) => match if compiled {
                    file_type.generate_compiled_filepath(&document.file)
                } else {
                    Some(document.file)
                } {
                    Some(ref file_path) => match file_type.load(file_path).await {
                        Ok(buffer) => match zip.write_all(&buffer) {
                            Ok(_) => {}
                            Err(e) => {
                                return ExportCompilerResult::Error(format!(
                                    "Error making a ZIP file: {:#?}",
                                    e
                                ));
                            }
                        },
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

    ExportCompilerResult::Success(bytes.get_ref().to_vec())
}

pub async fn merge_documents<F: FileProvider + ?Sized>(
    file_type: Arc<Box<F>>,
    mut documents: Vec<Document>,
    compiled: bool,
) -> ExportCompilerResult {
    if documents.len() == 1 {
        let document = documents.pop().unwrap();
        if let Some(ref file_path) = if compiled {
            file_type.generate_compiled_filepath(&document.file)
        } else {
            Some(document.file)
        } {
            match file_type.load(file_path).await {
                Ok(buffer) => match PdfDocument::load_mem(&buffer) {
                    Ok(mut document) => get_document_buffer(&mut document),
                    Err(e) => {
                        sentry::capture_error(&e);

                        ExportCompilerResult::Error(format!("Error loading the PDF: {:#?}", e))
                    }
                },
                Err(error) => match error {
                    FileError::IoError(e) => {
                        sentry::capture_error(&e);

                        ExportCompilerResult::Error(format!("Error loading the PDF: {:#?}", e))
                    }
                    FileError::S3Error(e) => {
                        sentry::capture_error(&e);

                        ExportCompilerResult::Error(format!("Error loading the PDF: {:#?}", e))
                    }
                },
            }
        } else {
            ExportCompilerResult::Error("Error getting the compiled PDF file.".to_string())
        }
    } else {
        let documents_objects = processor::get_documents_containers(file_type, documents, compiled);
        if documents_objects.pages.is_empty() || documents_objects.objects.is_empty() {
            ExportCompilerResult::Error("Cannot extract PDFs documents".into())
        } else if let Some(mut document) = processor::process_documents(documents_objects) {
            get_document_buffer(&mut document)
        } else {
            ExportCompilerResult::Error("Error decoding the PDFs files.".to_string())
        }
    }
}

fn get_document_buffer(document: &mut PdfDocument) -> ExportCompilerResult {
    let buf = Vec::<u8>::new();
    let mut cursor = Cursor::new(buf);

    match document.save_to(&mut cursor) {
        Ok(_) => {
            let _ = cursor.seek(SeekFrom::Start(0));

            ExportCompilerResult::Success(cursor.get_ref().to_vec())
        }
        Err(e) => ExportCompilerResult::Error(format!(
            "An error {:#?} occurred saving the PDFs files.",
            e
        )),
    }
}

async fn save_compiled_file<F: FileProvider + ?Sized>(
    file_type: Arc<Box<F>>,
    file_path: String,
    buf: Vec<u8>,
) -> HandlerCompilerResult {
    match file_type.save(file_path.as_str(), buf).await {
        FileResult::Saved => HandlerCompilerResult::Success,
        FileResult::Error(e) => {
            sentry::capture_error(&e);

            HandlerCompilerResult::Error(format!("Error {:#?} saving a PDF file, aborted.", e))
        }
        FileResult::BlockingError(e) => {
            sentry::capture_error(&e);

            HandlerCompilerResult::Error(format!("Error {:#?} saving a PDF file, aborted.", e))
        }
        FileResult::S3Error(e) => {
            sentry::capture_error(&e);

            HandlerCompilerResult::Error(format!("Error {:#?} saving a PDF file, aborted.", e))
        }
        _ => HandlerCompilerResult::Error("Error: cannot save the file.".into()),
    }
}
