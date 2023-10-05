mod info;
mod conv;
mod console;
mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(msg: &str);
}

#[wasm_bindgen]
pub fn replace_rgb_colors(image: Vec<u8>, colors: Vec<JsValue>) -> Result<JsValue, JsValue> {
    conv::replace_rgb_colors(
        &image,
        &colors.iter().map(|entry| entry.clone().into()).collect()
    ).map_or_else(
        |err| Err(err.into()),
        |ok| Ok(ok.into())
    )
}

#[wasm_bindgen]
pub fn image_info(image: Vec<u8>) -> Result<JsValue, JsValue> {
    info::image_info(&image).map_or_else(|err| Err(err.into()), |ok| Ok(ok.into()))
}