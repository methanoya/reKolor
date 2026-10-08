//! The browser interface over `rekolor-core` (T1–T7): a thin adapter that converts between
//! TypeScript-facing types and core types. No color logic lives here.
//!
//! Interface rules:
//! - pixel buffers cross as typed arrays (`Uint8Array`), with lengths checked in Rust (T3);
//! - structured values cross as tsify types wrapped in `Ts<T>`, converted *inside* each
//!   function, so malformed input becomes an error value instead of a trap or a leak (T2);
//! - domain errors are returned as `Outcome<T>` values, never thrown (T5);
//! - field names are camelCase on the TypeScript side (T6).
//!
//! Must never depend on `rekolor-io` (that would pull the `image` crate into the WASM build).

mod config;
mod image;
mod outcome;
mod palette;
mod types;

pub use config::{
    ConfigExport, ConfigPick, ConfigSection, ConfigText, ConfigUnprinted, ParsedConfig,
    ResolvedPick, ResolvedPicks, parse_config, serialize_config,
};
pub use image::SourceImageOutcome;
pub use image::{SourceImage, composite};
pub use outcome::{
    ColorCountOutcome, ErrorInfo, ErrorKind, ImageStatsOutcome, NearestOutcome, Outcome,
    PickOutcome, RecolorOutcome, RgbOutcome, SuggestOutcome,
};
pub use palette::Palette;
pub use palette::PaletteOutcome;
pub use types::{
    ColorCount, ColorMismatch, ImageStats, Mapping, MaterialRange, PaletteData, PaletteEntry,
    PaletteMatch, PaletteMatches, Pick, RecolorRequest, RecolorStats, Rgb, Rgba,
};

use wasm_bindgen::prelude::*;

/// Runs once when the module is instantiated: Rust panics (bugs, never input errors) are printed
/// with their message to `console.error`, in production too (R8); `log` traces go to the console
/// at warning level and above (R7).
#[wasm_bindgen(start, skip_typescript)]
fn start() {
    console_error_panic_hook::set_once();
    // Fails only if a logger is already installed; nothing to do then.
    let _ = console_log::init_with_level(log::Level::Warn);
}
