use std::collections::BTreeMap;
use std::str;

use lopdf::{Dictionary, Document as PdfDocument, Object, ObjectId};

use crate::file::File;
use crate::logger::Logger;
use crate::redis::models::document::Document;

const PDF_VERSION: &'static str = "1.5";

pub struct DocumentObjects {
    pub objects: BTreeMap<ObjectId, Object>,
    pub pages: BTreeMap<ObjectId, Object>,
}

pub fn get_documents_containers(documents: Vec<Document>) -> DocumentObjects {
    let mut max_id = 1;

    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();

    for document in documents {
        if let Some(ref file_name) = File::get_compiled_filepath(&document.file) {
            match PdfDocument::load(file_name) {
                Ok(mut document) => {
                    document.renumber_objects_with(max_id);

                    max_id = document.max_id + 1;

                    documents_pages.extend(
                        document
                            .get_pages()
                            .into_iter()
                            .map(|(_, object_id)| {
                                (
                                    object_id,
                                    document.get_object(object_id).unwrap().to_owned(),
                                )
                            })
                            .collect::<BTreeMap<ObjectId, Object>>(),
                    );
                    documents_objects.extend(document.objects);
                }
                Err(e) => {
                    sentry::capture_error(&e);

                    Logger::log(format!("Error loading the PDF: {:#?}", e));
                }
            }
        }
    }

    DocumentObjects {
        pages: documents_pages,
        objects: documents_objects,
    }
}

pub fn process_documents(documents_objects: DocumentObjects) -> Option<PdfDocument> {
    let mut document = PdfDocument::with_version(PDF_VERSION);

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.objects.iter() {
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        *object_id
                    },
                    object.clone(),
                ));
            }
            "Pages" => {
                if let Some(dictionary) =
                    upsert_dictionary(&object, pages_object.as_ref().map(|(_, object)| object))
                {
                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            *object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" => {}
            "Outlines" => {}
            "Outline" => {}
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    if pages_object.is_none() {
        return None;
    }

    for (object_id, object) in documents_objects.pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.as_ref().unwrap().0);

            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    if catalog_object.is_none() {
        return None;
    }

    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();

    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_objects.pages.len() as u32);

        document
            .objects
            .insert(pages_object.0, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines"); // Outlines not supported in merged PDFs

        document
            .objects
            .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    document.max_id = document.objects.len() as u32;

    document.renumber_objects();
    document.compress();

    Some(document)
}

fn upsert_dictionary(object: &Object, other_object: Option<&Object>) -> Option<Dictionary> {
    if let Ok(dictionary) = object.as_dict() {
        let mut dictionary = dictionary.clone();
        if let Some(object) = other_object {
            if let Ok(old_dictionary) = object.as_dict() {
                dictionary.extend(old_dictionary);
            }
        }

        Some(dictionary)
    } else {
        None
    }
}
