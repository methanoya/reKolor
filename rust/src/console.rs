use web_sys;
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
pub fn log_1(msg: &JsValue) {
    web_sys::console::log_1(msg);
}
#[cfg(not(target_arch = "wasm32"))]
pub fn log_1(msg: &String) {
    println!("{:?}", msg);
}