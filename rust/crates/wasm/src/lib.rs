//! The browser interface over `rekolor-core`: a thin adapter that converts between
//! TypeScript-facing types and core types. No color logic lives here.
//!
//! Interface rules:
//! - pixel buffers cross as typed arrays (`Uint8Array`), with lengths checked in Rust;
//! - structured values cross as tsify types wrapped in `Ts<T>`, converted *inside* each
//!   function, so malformed input becomes an error value instead of a trap or a leak;
//! - domain errors are returned as `Outcome<T>` values, never thrown;
//! - field names are camelCase on the TypeScript side.
//!
//! Must never depend on `rekolor-io` (that would pull the `image` crate into the WASM build).

// How the browser uses this crate: `wasm-pack build` compiles it to a `.wasm` file plus generated
// JavaScript glue and TypeScript declarations (`pkg/`). The web app depends on that folder as the
// npm package `rekolor-wasm`; its Web Worker loads the module and calls it through `engine.ts`.
// Everything marked `#[wasm_bindgen]` in the modules below is visible to JavaScript; everything
// else stays internal to the WASM module.
//
// Private modules (no `pub`): their public items are re-exported below instead.
mod config;
mod image;
mod outcome;
mod palette;
mod types;

// The public surface, gathered in one place. `pub use` re-exports items from the private modules,
// so Rust tests can import them as `rekolor_wasm::Name`.
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

// `prelude::*` imports the names every wasm-bindgen user needs (`wasm_bindgen`, `JsValue`, …).
use wasm_bindgen::prelude::*;

// `start` makes wasm-bindgen call this function automatically when the module loads;
// `skip_typescript` keeps it out of the TypeScript declarations (nobody calls it by hand).
/// Runs once when the module is instantiated: Rust panics (bugs, never input errors) are printed
/// with their message to `console.error`, in production too; `log` traces go to the console
/// at warning level and above.
#[wasm_bindgen(start, skip_typescript)]
fn start() {
    console_error_panic_hook::set_once();
    // Fails only if a logger is already installed; nothing to do then.
    // `let _ = ...` explicitly ignores the returned `Result`.
    let _ = console_log::init_with_level(log::Level::Warn);
}
