//! The browser interface over `rekolor-core` (T1–T7): a thin adapter that converts between
//! TypeScript-facing types and core types. No color logic lives here.
//!
//! Must never depend on `rekolor-io` (that would pull the `image` crate into the WASM build).
