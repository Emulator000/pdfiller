use std::str;

use actix_multipart::Field;

use futures_lite::stream::StreamExt;

use uuid::Uuid;

pub const PATH: &'static str = "./tmp/";
pub const PATH_COMPILED: &'static str = "./tmp/compiled/";

pub async fn read_chunked_string(field: &mut Field) -> String {
    let mut string = String::new();
    while let Some(chunk) = field.next().await {
        match chunk {
            Ok(data) => {
                string.push_str(str::from_utf8(&data).unwrap_or(""));
            }
            Err(e) => {
                sentry::capture_error(&e);
            }
        }
    }

    string
}

pub fn file_path<S: AsRef<str>>(filename: S) -> String {
    format!(
        "{}{}{}",
        PATH,
        Uuid::new_v4().to_string(),
        sanitize_filename::sanitize(filename.as_ref())
    )
}
