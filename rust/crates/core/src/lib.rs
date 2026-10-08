//! Color matching and recoloring on raw RGBA8 buffers (R1, R3, R4).
//!
//! Pure functions only: no file I/O, no printing, and no dependency on wasm-bindgen, tsify or
//! `image`. Decoding, encoding and the browser boundary live in the other workspace crates.
//! Optional debug traces go through the `log` facade (R7).
//!
//! Behavior reproduces the existing implementation exactly (R9), checked against the recorded
//! baseline in `rust/testdata/baseline/`. Known behavior issues are changed later, one decision
//! at a time.

mod analyze;
mod color;
mod error;
mod image;
mod palette;
mod pick;
mod recolor;

pub use analyze::{ImageStats, analyze, color_count};
pub use color::{Lab, Rgb8, Rgba8, composite, delta_e_2000, delta_e_2000_lab, lab};
pub use error::Error;
pub use image::ImageRef;
pub use palette::{NON_PALETTE_MARKER, Palette, PaletteEntry, PaletteMatch};
pub use pick::{ColorMismatch, PICK_MISMATCH_TOLERANCE, Pick, pick};
pub use recolor::{Mapping, MaterialRange, RecolorStats, recolor, recolor_with_ranges};
