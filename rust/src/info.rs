use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use image::{GenericImageView, Pixel, Rgb, Rgba};
use typeshare::typeshare;
use wasm_bindgen::JsValue;
use crate::utils;

#[typeshare]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub rgb_colors: u32,
    pub real_colors: u32,
}

impl Into<JsValue> for ImageInfo {
    fn into(self) -> JsValue {
        serde_wasm_bindgen::to_value(&self).unwrap()
    }
}

// impl From<ImageInfo> for JsValue {
//     fn from(value: ImageInfo) -> Self {
//         serde_wasm_bindgen::to_value(&value).unwrap()
//     }
// }

pub fn image_info(image: &Vec<u8>) -> Result<ImageInfo, String> {
    match utils::read(image) {
        Ok(image) => {
            let (width, height) = image.dimensions();
            let mut rgb_colors: HashSet<Rgb<u8>> = HashSet::new();
            let mut real_colors: HashSet<Rgba<u8>> = HashSet::new();
            for (_x, _y, pixel) in image.pixels() {
                rgb_colors.insert(pixel.to_rgb());
                real_colors.insert(pixel);
            }
            Ok(ImageInfo {
                width: width,
                height: height,
                rgb_colors: rgb_colors.len() as u32,
                real_colors: real_colors.len() as u32,
            })
        }
        Err(err) => Err(err.to_string())
    }
}
