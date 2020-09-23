use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str;

use lopdf::{Document, Object, ObjectId};

use uuid::Uuid;

pub const PATH: &'static str = "./tmp/";
pub const PATH_COMPILED: &'static str = "./tmp/compiled/";

pub fn file_path<S: AsRef<str>>(filename: S) -> String {
    format!(
        "{}{}{}",
        PATH,
        Uuid::new_v4().to_string(),
        sanitize_filename::sanitize(filename.as_ref())
    )
}

pub fn get_filename<S: AsRef<str>>(filename: S) -> Option<String> {
    match Path::new(filename.as_ref()).file_name() {
        Some(file_name) => match file_name.to_str() {
            Some(file_name) => Some(file_name.into()),
            None => None,
        },
        None => None,
    }
}

pub fn read_file_string<S: AsRef<str>>(path: S) -> Option<String> {
    let path = Path::new(path.as_ref());
    match File::open(&path) {
        Err(_) => None,
        Ok(mut file) => {
            let mut buffer = String::new();
            match file.read_to_string(&mut buffer) {
                Ok(_) => Some(buffer),
                Err(_) => None,
            }
        }
    }
}

pub fn read_file_buf<S: AsRef<str>>(path: S) -> Option<Vec<u8>> {
    let path = Path::new(path.as_ref());
    match File::open(&path) {
        Err(_) => None,
        Ok(mut file) => {
            let mut buffer = Vec::new();
            match file.read_to_end(&mut buffer) {
                Ok(_) => Some(buffer),
                Err(_) => None,
            }
        }
    }
}

pub trait CustomDocumentRenumber {
    fn renumber_objects_from_id(&mut self, new_id: u32);
}

impl CustomDocumentRenumber for Document {
    fn renumber_objects_from_id(&mut self, new_id: u32) {
        let mut replace = BTreeMap::new();
        let mut new_id = new_id;
        let mut ids = self.objects.keys().cloned().collect::<Vec<ObjectId>>();
        ids.sort();

        for id in ids {
            if id.0 != new_id {
                replace.insert(id, (new_id, id.1));
            }

            new_id += 1;
        }

        let mut objects = BTreeMap::new();
        for (old, new) in &replace {
            if let Some(object) = self.objects.remove(old) {
                objects.insert(new.clone(), object);
            }
        }

        for (new, object) in objects {
            self.objects.insert(new, object);
        }

        let action = |object: &mut Object| {
            if let Object::Reference(ref mut id) = *object {
                if replace.contains_key(&id) {
                    *id = replace[id];
                }
            }
        };

        self.traverse_objects(action);

        self.max_id = new_id - 1;
    }
}
