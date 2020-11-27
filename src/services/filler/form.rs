use std::collections::HashMap;
use std::str;

use serde_json::Value;

use pdf_forms::{FieldState, Form, LoadError, ValueError};

use lopdf::xobject;

use regex::Regex;

use crate::client;
use crate::mongo::models::document::Document;
use crate::utils;

pub type PDFillerMap = HashMap<String, Value>;

const REQUIRED_MARKER: char = '!';
const IMAGE_REGEX: &str = r"_af_image$";

#[derive(Debug)]
pub enum FillingError {
    Load(LoadError),
    Value(ValueError),
    RequiredField(String),
    InternalError,
}

pub async fn fields_filler(map: &PDFillerMap, document: &Document) -> Result<Form, FillingError> {
    match Form::load(&document.file) {
        Ok(mut form) => {
            for (index, name) in form.get_all_names().iter().enumerate() {
                if let Some(name) = name {
                    let name = name.trim_start_matches(REQUIRED_MARKER);

                    let mut value = map.get(name);
                    let result = {
                        if value.is_some() {
                            match form.get_state(index) {
                                FieldState::Text { required, .. } => {
                                    if required && value.is_none() {
                                        Err(FillingError::RequiredField(name.to_owned()))
                                    } else if let Some(value) = value {
                                        form.set_text(index, value.as_str().unwrap_or("").into())
                                            .map_err(FillingError::Value)
                                    } else {
                                        Ok(())
                                    }
                                }
                                FieldState::Radio { required, .. } => {
                                    if required && value.is_none() {
                                        Err(FillingError::RequiredField(name.to_owned()))
                                    } else if let Some(value) = value {
                                        form.set_radio(index, value.as_str().unwrap_or("").into())
                                            .map_err(FillingError::Value)
                                    } else {
                                        Ok(())
                                    }
                                }
                                FieldState::CheckBox { required, .. } => {
                                    if required && value.is_none() {
                                        Err(FillingError::RequiredField(name.to_owned()))
                                    } else if let Some(value) = value {
                                        form.set_check_box(index, value.as_bool().unwrap_or(false))
                                            .map_err(FillingError::Value)
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
                                                .map_err(FillingError::Value),
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
                                                .map_err(FillingError::Value),
                                            None => Ok(()),
                                        }
                                    } else {
                                        Ok(())
                                    }
                                }
                                _ => Ok(()),
                            }
                        } else {
                            // This is needed as the current regex is a bit unuseful
                            #[allow(clippy::trivial_regex)]
                            let image_regex = Regex::new(IMAGE_REGEX)
                                .map_err(|_err| FillingError::InternalError)?;

                            value = map.get(image_regex.replace(name, "").as_ref());

                            if let Some(uri) = value {
                                let object_id = form.get_object_id(index);
                                if let Ok(page_id) = form.document.get_object_page(object_id) {
                                    if let Some(image) =
                                        client::get(uri.as_str().unwrap_or("")).await
                                    {
                                        if let Ok(object) = form.document.get_object(object_id) {
                                            if let Ok(dict) = object.as_dict() {
                                                if let Ok(rect) = utils::get_object_rect(dict) {
                                                    if let Ok(stream) = xobject::image_from(image) {
                                                        let _ = form.document.insert_image(
                                                            page_id,
                                                            stream,
                                                            (rect.0, rect.1),
                                                            (rect.3, rect.2),
                                                        );

                                                        let _ = form.remove_field(index);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            Ok(())
                        }
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
