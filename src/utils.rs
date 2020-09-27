use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::str;

use lopdf::Dictionary;

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

pub fn get_object_rect(field: &Dictionary) -> Result<(f64, f64, f64, f64), lopdf::Error> {
    let rect = field
        .get(b"Rect")?
        .as_array()?
        .iter()
        .map(|object| {
            object
                .as_f64()
                .unwrap_or(object.as_i64().unwrap_or(0) as f64)
        })
        .collect::<Vec<_>>();

    if rect.len() == 4 {
        Ok((rect[0], rect[1], rect[2], rect[3]))
    } else {
        Err(lopdf::Error::ObjectNotFound)
    }
}
