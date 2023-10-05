#[cfg(target_arch = "wasm32")]
mod module {
    use web_sys;
    use wasm_bindgen::JsValue;
    pub fn log_1(msg: &JsValue) {
        web_sys::console::log_1(msg);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod module {
    pub fn log_1(msg: &String) {
        println!("{:?}", msg);
    }
}

pub use module::*;
