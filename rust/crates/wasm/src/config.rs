//! Palette config import/export for the web app (W10 v): thin wrappers over `rekolor-config`, so
//! the app reads, validates and writes `*.palettes.toml` exactly like the CLI.

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
pub struct ParsedConfig {
    /// In file order.
    pub sections: Vec<ConfigSection>,
}

/// A config pick resolved against the palette.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
pub struct ResolvedPick {
    /// The stored pixel color from the config.
    pub pixel: Rgba,
    /// `pixel` composited over white: what recolor matches (computed in Rust, as for a click).
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ConfigExport {
    /// The image's file name, mentioned in the header.
    pub image_name: String,
    pub picks: Vec<ConfigPick>,
}

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

impl From<ConfigPick> for config::ConfigPick {
    fn from(p: ConfigPick) -> Self {
        let Rgba { r, g, b, a } = p.rgba;
        Self {
            rgba: [r, g, b, a],
            ink: p.ink,
        }
    }
}

/// Parses and validates a palette config (sizes, distinct inks per size, limits). Ink names are
/// checked later against a palette (`Palette.resolveSection`).
#[wasm_bindgen(js_name = parseConfig)]
pub fn parse_config(text: &str) -> ParsedConfigOutcome {
    let outcome = config::PaletteConfig::parse(text)
        .map(|c| ParsedConfig {
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
            let size = u32::try_from(request.picks.len()).unwrap_or(u32::MAX);
            let config = config::PaletteConfig {
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
    pub(crate) fn resolve(&self, section: ConfigSection) -> Result<ResolvedPicks, ErrorInfo> {
        let sized = config::SizedPalette {
            size: section.size,
            picks: section.picks.into_iter().map(Into::into).collect(),
        };
        sized.validate()?;
        let palette = self.core();
        let picks = sized
            .resolve(palette)?
            .into_iter()
            .map(|p| {
                let entry = &palette.entries()[p.index];
                ResolvedPick {
                    pixel: p.rgba.into(),
                    matching: p.matching.into(),
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

#[wasm_bindgen]
impl Palette {
    /// Resolves a config section against this palette: every ink must be a palette entry (an
    /// unknown name is an `invalidConfig` error; nothing is skipped), and each pick's matching
    /// color is its pixel composited over white. Picks keep their order.
    #[wasm_bindgen(js_name = resolveSection)]
    pub fn resolve_section(&self, section: Ts<ConfigSection>) -> ResolvedPicksOutcome {
        let outcome = section
            .to_rust()
            .map_err(ErrorInfo::from)
            .and_then(|s| self.resolve(s));
        outcome_js(Outcome::from(outcome))
    }
}
