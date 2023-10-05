use image::io::Reader;
use image::{DynamicImage, ImageError, };
use web_sys::console;
use std::io::Cursor;

pub fn read(input: &Vec<u8>) -> Result<DynamicImage, ImageError> {
    match Reader::new(Cursor::new(input)).with_guessed_format() {
        Ok(raw) => match raw.decode() {
            Ok(value) => Ok(value),
            Err(err) => {
                console::log_2(&"Decoding error:".into(), &err.to_string().as_str().into());
                Err(err)
            }
        }
        Err(err) => {
            console::log_2(&"Reading error:".into(), &err.to_string().as_str().into());
            Err(ImageError::from(err))
        }
    }
}
