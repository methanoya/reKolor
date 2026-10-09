//! Color matching and recoloring on raw RGBA8 buffers.
//!
//! An image here is a flat list of bytes, four per pixel (red, green, blue, alpha, each 0–255),
//! row by row from the top left: "RGBA8". The engine composites each pixel over the material
//! color, compares colors in the CIE Lab color space with the CIEDE2000 difference (ΔE, roughly
//! "how different two colors look"), and replaces pixels with ink colors from a palette.
//!
//! Pure functions only: no file I/O, no printing, and no dependency on wasm-bindgen, tsify or
//! `image`. Decoding, encoding and the browser boundary live in the other workspace crates.
//! Optional debug traces go through the `log` facade.
//!
//! Behavior is checked against the recorded baseline in `rust/testdata/baseline/`.

// A Rust crate is split into modules; `mod name;` includes the file `name.rs` from this folder.
// Modules are private unless re-exported below, so callers only see what this file lists.
mod analyze;
mod color;
mod error;
mod image;
mod palette;
mod pick;
mod recolor;

// `pub use` re-exports items, so callers write `rekolor_core::Palette` instead of the full path
// `rekolor_core::palette::Palette`. This list is the crate's public API.
pub use analyze::{ImageStats, analyze, color_count};
pub use color::{Lab, Rgb8, Rgba8, composite, delta_e_2000, delta_e_2000_lab, lab};
pub use error::Error;
pub use image::ImageRef;
pub use palette::{NON_PALETTE_MARKER, Palette, PaletteEntry, PaletteMatch};
pub use pick::{ColorMismatch, PICK_MISMATCH_TOLERANCE, Pick, pick};
pub use recolor::{
    Mapping, MaterialRange, RecolorStats, is_unprinted, recolor, recolor_with_ranges,
};
