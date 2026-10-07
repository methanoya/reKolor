use rekolor_core as core;
use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

/// The result of every call that can fail on input (T5):
/// `{ status: "ok", value }` or `{ status: "error", error }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum Outcome<T> {
    Ok { value: T },
    Error { error: ErrorInfo },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ErrorInfo {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// Width or height is 0.
    EmptyImage,
    /// Width × height can't be addressed.
    TooLarge,
    /// The pixel buffer isn't width × height × 4 bytes.
    BufferLength,
    /// The output buffer doesn't match the image.
    OutputLength,
    /// A pick coordinate is outside the image.
    OutOfBounds,
    /// The palette has no entries.
    EmptyPalette,
    /// A structured argument doesn't have the expected shape.
    InvalidInput,
}

impl From<core::Error> for ErrorInfo {
    fn from(e: core::Error) -> Self {
        let kind = match e {
            core::Error::EmptyImage { .. } => ErrorKind::EmptyImage,
            core::Error::TooLarge { .. } => ErrorKind::TooLarge,
            core::Error::BufferLength { .. } => ErrorKind::BufferLength,
            core::Error::OutputLength { .. } => ErrorKind::OutputLength,
            core::Error::OutOfBounds { .. } => ErrorKind::OutOfBounds,
            core::Error::EmptyPalette => ErrorKind::EmptyPalette,
            _ => ErrorKind::InvalidInput,
        };
        Self {
            kind,
            message: e.to_string(),
        }
    }
}

impl From<tsify::Error> for ErrorInfo {
    fn from(e: tsify::Error) -> Self {
        Self {
            kind: ErrorKind::InvalidInput,
            message: e.to_string(),
        }
    }
}

impl<T> From<Result<T, ErrorInfo>> for Outcome<T> {
    fn from(result: Result<T, ErrorInfo>) -> Self {
        match result {
            Ok(value) => Outcome::Ok { value },
            Err(error) => Outcome::Error { error },
        }
    }
}

#[wasm_bindgen]
extern "C" {
    // tsify names a generic type without its argument (`Outcome`), so each method returns one of
    // these instead, which the `.d.ts` shows with the precise type.
    #[wasm_bindgen(typescript_type = "Outcome<RecolorStats>")]
    pub type RecolorOutcome;
    #[wasm_bindgen(typescript_type = "Outcome<Pick>")]
    pub type PickOutcome;
    #[wasm_bindgen(typescript_type = "Outcome<PaletteMatch>")]
    pub type SuggestOutcome;
    #[wasm_bindgen(typescript_type = "Outcome<PaletteMatches>")]
    pub type NearestOutcome;
}

/// Converts an outcome to its TypeScript form, typed as `O` (one of the types above).
pub(crate) fn outcome_js<T: Tsify + Serialize, O: JsCast>(outcome: Outcome<T>) -> O {
    to_ts(&outcome).js_value().unchecked_into()
}

/// Converts a Rust value to its TypeScript form. Only called with plain data structs, which
/// always serialize; a failure would be a bug, not an input error.
pub(crate) fn to_ts<T: Tsify + Serialize>(value: &T) -> Ts<T> {
    Ts::from_rust(value).expect("plain data structs always serialize")
}

/// Builds `{ status: "ok", value }` around a JS value that serde can't produce, such as an
/// exported class instance (used by the `create` factories).
pub(crate) fn ok_object(value: JsValue) -> JsValue {
    let object = js_sys::Object::new();
    set(&object, "status", &"ok".into());
    set(&object, "value", &value);
    object.into()
}

/// Builds `{ status: "error", error }`.
pub(crate) fn error_object(error: &ErrorInfo) -> JsValue {
    let object = js_sys::Object::new();
    set(&object, "status", &"error".into());
    set(&object, "error", &to_ts(error).js_value());
    object.into()
}

fn set(object: &js_sys::Object, key: &str, value: &JsValue) {
    // Setting a property on a fresh plain object can't fail.
    js_sys::Reflect::set(object, &key.into(), value).expect("set property on a plain object");
}
