//! Library part of the reKolor command-line tool: palette and config files, config generation,
//! input discovery and golden outputs. The `rekolor` binary (`main.rs`) is a thin argument layer
//! on top; the golden tests reuse this library.

pub mod config;
pub mod discover;
pub mod generate;
pub mod palette_file;
