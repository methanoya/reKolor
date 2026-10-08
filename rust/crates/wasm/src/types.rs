//! TypeScript-facing data types (T2, T6) and their conversions to and from core types.

use rekolor_core as core;
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// One picked color and the ink that replaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct Mapping {
    pub source: Rgb,
    pub ink: Rgb,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct RecolorRequest {
    pub mappings: Vec<Mapping>,
    /// The material color the image is composited over (the garment or substrate). Required: white
    /// reproduces the behavior from before the material.
    pub material: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Tsify)]
pub struct RecolorStats {
    /// Pixels whose composited color equals a mapping's source exactly.
    pub exact: f64,
    /// All other pixels.
    pub nearest: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ImageStats {
    pub width: u32,
    pub height: u32,
    /// Distinct colors after compositing over the material: what `recolor` matches.
    pub colors: f64,
    /// Distinct RGBA values.
    pub rgba_colors: f64,
}

/// The number of distinct colors after compositing over the material (`SourceImage.colorCount`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Tsify)]
pub struct ColorCount {
    /// At most 2^24, exact as a JS number.
    pub colors: f64,
}

/// One palette color as passed in, e.g. from `palettes/pantone.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct PaletteEntry {
    pub name: String,
    pub rgb: Rgb,
}

/// An **ordered** list of palette entries; the order decides ties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct PaletteData {
    pub entries: Vec<PaletteEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PaletteMatch {
    /// Position in the palette; identifies the entry even where RGB values repeat.
    pub index: u32,
    pub name: String,
    pub rgb: Rgb,
    /// Not a real ink (e.g. "Pure White (non-palette)").
    pub non_palette: bool,
    pub delta_e: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct PaletteMatches {
    pub matches: Vec<PaletteMatch>,
}

/// A warning: the color the caller saw differs from the stored pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ColorMismatch {
    pub seen: Rgba,
    pub stored: Rgba,
    pub max_channel_difference: u8,
}

/// The result of picking a pixel (R10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct Pick {
    /// The stored pixel.
    pub pixel: Rgba,
    /// The color used for matching (the pixel composited over the material).
    pub matching: Rgb,
    /// The suggested palette entry.
    pub suggestion: PaletteMatch,
    /// Present when the color the caller saw differs from the stored pixel beyond the
    /// tolerance. A warning only.
    #[tsify(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mismatch: Option<ColorMismatch>,
}

impl From<Rgb> for core::Rgb8 {
    fn from(Rgb { r, g, b }: Rgb) -> Self {
        Self::new(r, g, b)
    }
}

impl From<core::Rgb8> for Rgb {
    fn from(c: core::Rgb8) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}

impl From<Rgba> for core::Rgba8 {
    fn from(Rgba { r, g, b, a }: Rgba) -> Self {
        Self::new(r, g, b, a)
    }
}

impl From<core::Rgba8> for Rgba {
    fn from(c: core::Rgba8) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }
}

impl From<Mapping> for core::Mapping {
    fn from(m: Mapping) -> Self {
        Self {
            source: m.source.into(),
            ink: m.ink.into(),
        }
    }
}

impl From<core::RecolorStats> for RecolorStats {
    fn from(s: core::RecolorStats) -> Self {
        // Counts are at most width × height, far below 2^53, so they're exact as JS numbers.
        Self {
            exact: s.exact as f64,
            nearest: s.nearest as f64,
        }
    }
}

impl From<core::ImageStats> for ImageStats {
    fn from(s: core::ImageStats) -> Self {
        Self {
            width: s.width,
            height: s.height,
            colors: s.colors as f64,
            rgba_colors: s.rgba_colors as f64,
        }
    }
}

impl From<core::PaletteMatch<'_>> for PaletteMatch {
    fn from(m: core::PaletteMatch<'_>) -> Self {
        Self {
            index: m.index as u32,
            name: m.entry.name.clone(),
            rgb: m.entry.rgb.into(),
            non_palette: m.entry.non_palette,
            delta_e: m.delta_e,
        }
    }
}

impl From<core::ColorMismatch> for ColorMismatch {
    fn from(m: core::ColorMismatch) -> Self {
        Self {
            seen: m.seen.into(),
            stored: m.stored.into(),
            max_channel_difference: m.max_channel_difference,
        }
    }
}

impl From<core::Pick<'_>> for Pick {
    fn from(p: core::Pick<'_>) -> Self {
        Self {
            pixel: p.pixel.into(),
            matching: p.matching.into(),
            suggestion: p.suggestion.into(),
            mismatch: p.mismatch.map(Into::into),
        }
    }
}
