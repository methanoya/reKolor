//! Palette config import/export for the web app: thin wrappers over `rekolor-config`, so
//! the app reads, validates and writes `*.palettes.toml` exactly like the CLI.

// `use ... as ...` gives a crate a short local name: `config::PaletteConfig` below is the shared
// config crate. The structs in this file mirror its types for TypeScript, like `types.rs` does for
// the core.
use rekolor_config as config;
use rekolor_core as core;
use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

use crate::outcome::outcome_js;
use crate::{ErrorInfo, ErrorKind, Outcome, Palette, PaletteMatch, Rgb, Rgba};

/// One pick in a config: the stored pixel color and its ink's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ConfigPick {
    pub rgba: Rgba,
    pub ink: String,
}

/// One `[[palette]]` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ConfigSection {
    /// The requested number of distinct inks.
    pub size: u32,
    pub picks: Vec<ConfigPick>,
}

// In JavaScript: `{ kind: "color", rgba, deltaE }` or `{ kind: "material", deltaE }`.
// `tag = "kind"` adds the variant name as a `kind` field, and the inner `rename_all` writes
// `delta_e` as `deltaE`.
/// A color left unprinted, as in a config: a stored pixel color, or the
/// material's own color (whatever the material is).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConfigUnprinted {
    #[serde(rename_all = "camelCase")]
    Color { rgba: Rgba, delta_e: f32 },
    #[serde(rename_all = "camelCase")]
    Material { delta_e: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct ParsedConfig {
    /// The material color, for every section; white when the file has no `material` line.
    pub material: Rgb,
    /// Colors left unprinted, for every section, in file order; none when the file has no
    /// `unprinted` line.
    pub unprinted: Vec<ConfigUnprinted>,
    /// In file order.
    pub sections: Vec<ConfigSection>,
}

/// A config pick resolved against the palette.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct ResolvedPick {
    /// The stored pixel color from the config.
    pub pixel: Rgba,
    /// `pixel` composited over the material: what recolor matches (computed in Rust, as for a
    /// click).
    pub matching: Rgb,
    /// The named ink, with its distance from `matching`.
    pub ink: PaletteMatch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct ResolvedPicks {
    /// In config order.
    pub picks: Vec<ResolvedPick>,
}

/// What to export: the current picks, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ConfigExport {
    /// The image's file name, mentioned in the header.
    pub image_name: String,
    /// The material the picks were made on; always written.
    pub material: Rgb,
    /// Colors left unprinted, in order; always written (none if absent).
    #[tsify(optional)]
    #[serde(default)]
    pub unprinted: Vec<ConfigUnprinted>,
    pub picks: Vec<ConfigPick>,
}

// A plain wrapper, because an `Outcome` value must be an object (`{ text }`), not a bare string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ConfigText {
    pub text: String,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Outcome<ParsedConfig>")]
    pub type ParsedConfigOutcome;
    #[wasm_bindgen(typescript_type = "Outcome<ResolvedPicks>")]
    pub type ResolvedPicksOutcome;
    #[wasm_bindgen(typescript_type = "Outcome<ConfigText>")]
    pub type ConfigTextOutcome;
}

// Conversions between the config crate's types and the TypeScript-facing ones, in both
// directions (reading a file, and writing one).
impl From<config::ConfigError> for ErrorInfo {
    fn from(e: config::ConfigError) -> Self {
        Self {
            kind: ErrorKind::InvalidConfig,
            message: e.to_string(),
        }
    }
}

impl From<&config::ConfigPick> for ConfigPick {
    fn from(p: &config::ConfigPick) -> Self {
        Self {
            rgba: core::Rgba8::from(p.rgba).into(),
            ink: p.ink.clone(),
        }
    }
}

impl From<&config::Unprinted> for ConfigUnprinted {
    fn from(u: &config::Unprinted) -> Self {
        match *u {
            config::Unprinted::Color { rgba, delta_e } => ConfigUnprinted::Color {
                rgba: core::Rgba8::from(rgba).into(),
                delta_e,
            },
            config::Unprinted::Material { delta_e } => ConfigUnprinted::Material { delta_e },
        }
    }
}

impl From<ConfigUnprinted> for config::Unprinted {
    fn from(u: ConfigUnprinted) -> Self {
        match u {
            ConfigUnprinted::Color { rgba, delta_e } => config::Unprinted::Color {
                rgba: [rgba.r, rgba.g, rgba.b, rgba.a],
                delta_e,
            },
            ConfigUnprinted::Material { delta_e } => config::Unprinted::Material { delta_e },
        }
    }
}

impl From<ConfigPick> for config::ConfigPick {
    fn from(p: ConfigPick) -> Self {
        let Rgba { r, g, b, a } = p.rgba;
        Self {
            rgba: [r, g, b, a],
            ink: p.ink,
        }
    }
}

// `&str` parameters receive a JavaScript string (wasm-bindgen copies it in as UTF-8).
/// Parses and validates a palette config (sizes, distinct inks per size, limits). Ink names are
/// checked later against a palette (`Palette.resolveSection`).
#[wasm_bindgen(js_name = parseConfig)]
pub fn parse_config(text: &str) -> ParsedConfigOutcome {
    let outcome = config::PaletteConfig::parse(text)
        .map(|c| ParsedConfig {
            material: core::Rgb8::from(c.material).into(),
            unprinted: c.unprinted.iter().map(Into::into).collect(),
            sections: c
                .palette
                .iter()
                .map(|s| ConfigSection {
                    size: s.size,
                    picks: s.picks.iter().map(Into::into).collect(),
                })
                .collect(),
        })
        .map_err(ErrorInfo::from);
    outcome_js(Outcome::from(outcome))
}

/// Writes the picks as a config with one section (`size` = the number of picks).
#[wasm_bindgen(js_name = serializeConfig)]
pub fn serialize_config(request: Ts<ConfigExport>) -> ConfigTextOutcome {
    let outcome = request
        .to_rust()
        .map_err(ErrorInfo::from)
        .and_then(|request| {
            // The section's size is the number of picks. `try_from` fails only above 2^32 − 1
            // picks; `validate` below rejects far fewer anyway, so `u32::MAX` just guarantees that
            // error.
            let size = u32::try_from(request.picks.len()).unwrap_or(u32::MAX);
            let Rgb { r, g, b } = request.material;
            let config = config::PaletteConfig {
                material: [r, g, b],
                unprinted: request.unprinted.into_iter().map(Into::into).collect(),
                palette: vec![config::SizedPalette {
                    size,
                    picks: request.picks.into_iter().map(Into::into).collect(),
                }],
            };
            config.validate()?;
            let header = format!("Palette exported from the reKolor web app for {}.", request.image_name);
            Ok(ConfigText {
                text: config.to_toml(&[
                    &header,
                    "Each pick is a color as stored in the image (RGBA) and the ink it prints with.",
                ]),
            })
        });
    outcome_js(Outcome::from(outcome))
}

impl Palette {
    // Shared by `resolveSection` below and the Rust tests.
    pub(crate) fn resolve(
        &self,
        section: ConfigSection,
        material: Rgb,
    ) -> Result<ResolvedPicks, ErrorInfo> {
        let sized = config::SizedPalette {
            size: section.size,
            picks: section.picks.into_iter().map(Into::into).collect(),
        };
        sized.validate()?;
        let palette = self.core();
        let picks = sized
            .resolve(palette, material.into())?
            .into_iter()
            .map(|p| {
                let entry = &palette.entries()[p.index];
                ResolvedPick {
                    pixel: p.rgba.into(),
                    matching: p.matching.into(),
                    // Report the named ink as a match, with its real distance from the pick's
                    // color.
                    ink: PaletteMatch::from(core::PaletteMatch {
                        index: p.index,
                        entry,
                        delta_e: core::delta_e_2000(p.matching, entry.rgb),
                    }),
                }
            })
            .collect();
        Ok(ResolvedPicks { picks })
    }
}

// A second exported `impl` block for the same class: wasm-bindgen merges the methods, so
// JavaScript sees `palette.resolveSection(...)` next to `suggest` and `nearest`.
#[wasm_bindgen]
impl Palette {
    /// Resolves a config section against this palette: every ink must be a palette entry (an
    /// unknown name is an `invalidConfig` error; nothing is skipped), and each pick's matching
    /// color is its pixel composited over `material` (the config's, from `parseConfig`). Picks
    /// keep their order.
    #[wasm_bindgen(js_name = resolveSection)]
    pub fn resolve_section(
        &self,
        section: Ts<ConfigSection>,
        material: Ts<Rgb>,
    ) -> ResolvedPicksOutcome {
        let outcome = section.to_rust().map_err(ErrorInfo::from).and_then(|s| {
            let material = material.to_rust().map_err(ErrorInfo::from)?;
            self.resolve(s, material)
        });
        outcome_js(Outcome::from(outcome))
    }
}
