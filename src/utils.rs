use lopdf::Dictionary;

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
