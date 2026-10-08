//! Palette config files: `*.palettes.toml` (X1, W10 v). Shared by the CLI (golden set) and the web
//! app (import/export through `rekolor-wasm`), so both read and validate files the same way.
//!
//! ```toml
//! [[palette]]
//! size = 3
//! picks = [
//!   { rgba = [210, 120, 40, 255], ink = "Pantone 1595" },
//! ]
//! ```
//!
//! Each pick is a pixel color (straight-alpha RGBA, as stored in the image) and the palette entry it
//! maps to, by name. "Size" is the requested number of distinct inks; a palette may list fewer
//! picks ("up to N"), and several picks may share one ink.
//!
//! No file I/O and no dependency on `rekolor-io` or `wasm-bindgen`: callers read and write the text.

use std::collections::HashSet;

use rekolor_core::{Mapping, Palette, Rgb8, Rgba8, composite_over_white};
use serde::Deserialize;

/// Largest accepted config text (owner decision WA1 a).
pub const MAX_BYTES: usize = 256 * 1024;
/// Most `[[palette]]` sections in one config.
pub const MAX_SECTIONS: usize = 64;
/// Most picks in one section.
pub const MAX_PICKS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteConfig {
    pub palette: Vec<SizedPalette>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizedPalette {
    pub size: u32,
    pub picks: Vec<ConfigPick>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigPick {
    pub rgba: [u8; 4],
    pub ink: String,
}

/// A pick resolved against a palette: what the app and the recolorer need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPick {
    /// The stored pixel color from the config.
    pub rgba: Rgba8,
    /// The color the matcher sees: `rgba` composited over white (like an interactive pick).
    pub matching: Rgb8,
    /// The ink's position in the palette.
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("the config is {bytes} bytes; at most {max} are allowed", max = MAX_BYTES)]
    TooLarge { bytes: usize },
    #[error("{0}")]
    Syntax(String),
    #[error("the config has {count} palettes; at most {max} are allowed", max = MAX_SECTIONS)]
    TooManySections { count: usize },
    #[error("the size-{size} palette has {count} picks; at most {max} are allowed", max = MAX_PICKS)]
    TooManyPicks { size: u32, count: usize },
    #[error("a palette size appears more than once")]
    DuplicateSize,
    #[error("the size-{size} palette lists {inks} distinct inks (at most {size} allowed)")]
    TooManyInks { size: u32, inks: usize },
    #[error("no palette of size {0} in the config")]
    MissingSize(u32),
    #[error("ink {0:?} is not in the palette")]
    UnknownInk(String),
}

impl PaletteConfig {
    /// Parses and validates a config: size limits, no repeated sizes, at most `size` distinct inks
    /// per section. Ink names are checked against a palette later ([`SizedPalette::resolve`]).
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        if text.len() > MAX_BYTES {
            return Err(ConfigError::TooLarge { bytes: text.len() });
        }
        let config: PaletteConfig =
            toml::from_str(text).map_err(|e| ConfigError::Syntax(e.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// The structural rules of [`parse`](Self::parse), for configs built in code.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.palette.len() > MAX_SECTIONS {
            return Err(ConfigError::TooManySections {
                count: self.palette.len(),
            });
        }
        let mut sizes = HashSet::new();
        for palette in &self.palette {
            if !sizes.insert(palette.size) {
                return Err(ConfigError::DuplicateSize);
            }
            palette.validate()?;
        }
        Ok(())
    }

    pub fn size(&self, size: u32) -> Result<&SizedPalette, ConfigError> {
        self.palette
            .iter()
            .find(|p| p.size == size)
            .ok_or(ConfigError::MissingSize(size))
    }

    /// Writes the config in the simple format: each header line as a `# ` comment, then the
    /// sections in order.
    pub fn to_toml(&self, header: &[&str]) -> String {
        let mut out: String = header.iter().map(|line| format!("# {line}\n")).collect();
        for palette in &self.palette {
            out.push_str(&format!(
                "\n[[palette]]\nsize = {}\npicks = [\n",
                palette.size
            ));
            for pick in &palette.picks {
                let [r, g, b, a] = pick.rgba;
                let ink = basic_string(&pick.ink);
                out.push_str(&format!(
                    "  {{ rgba = [{r}, {g}, {b}, {a}], ink = {ink} }},\n"
                ));
            }
            out.push_str("]\n");
        }
        out
    }
}

/// A TOML basic string (`"…"`) with the required escapes.
fn basic_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl SizedPalette {
    /// Size N means up to N distinct inks; several picks may share one ink.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.picks.len() > MAX_PICKS {
            return Err(ConfigError::TooManyPicks {
                size: self.size,
                count: self.picks.len(),
            });
        }
        let inks: HashSet<&str> = self.picks.iter().map(|p| p.ink.as_str()).collect();
        if inks.len() > self.size as usize {
            return Err(ConfigError::TooManyInks {
                size: self.size,
                inks: inks.len(),
            });
        }
        Ok(())
    }

    /// Looks up every ink in `palette` (an unknown name is an error, nothing is skipped) and
    /// composites each pick over white, in pick order.
    pub fn resolve(&self, palette: &Palette) -> Result<Vec<ResolvedPick>, ConfigError> {
        self.picks
            .iter()
            .map(|pick| {
                let index = palette
                    .entries()
                    .iter()
                    .position(|e| e.name == pick.ink)
                    .ok_or_else(|| ConfigError::UnknownInk(pick.ink.clone()))?;
                let rgba = Rgba8::from(pick.rgba);
                Ok(ResolvedPick {
                    rgba,
                    matching: composite_over_white(rgba),
                    index,
                })
            })
            .collect()
    }

    /// The mappings for recoloring: each pick's matching color → its ink's color.
    pub fn mappings(&self, palette: &Palette) -> Result<Vec<Mapping>, ConfigError> {
        Ok(self
            .resolve(palette)?
            .into_iter()
            .map(|p| Mapping {
                source: p.matching,
                ink: palette.entries()[p.index].rgb,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rekolor_core::PaletteEntry;

    fn sample() -> PaletteConfig {
        PaletteConfig {
            palette: vec![
                SizedPalette {
                    size: 3,
                    picks: vec![
                        ConfigPick {
                            rgba: [210, 120, 40, 255],
                            ink: "Pantone 1595".into(),
                        },
                        ConfigPick {
                            rgba: [0, 0, 0, 0],
                            ink: "Pure White (non-palette)".into(),
                        },
                    ],
                },
                SizedPalette {
                    size: 7,
                    picks: vec![],
                },
            ],
        }
    }

    fn palette() -> Palette {
        Palette::new(vec![
            PaletteEntry::new("Pure White (non-palette)", Rgb8::new(255, 255, 255)),
            PaletteEntry::new("Pantone 1595", Rgb8::new(209, 91, 5)),
        ])
        .unwrap()
    }

    #[test]
    fn writes_the_simple_format_and_reads_it_back() {
        let text = sample().to_toml(&["Header line one.", "Two."]);
        assert!(text.starts_with("# Header line one.\n# Two.\n\n[[palette]]\nsize = 3\n"));
        assert!(text.contains("  { rgba = [210, 120, 40, 255], ink = \"Pantone 1595\" },\n"));
        assert_eq!(PaletteConfig::parse(&text).unwrap(), sample());
    }

    #[test]
    fn ink_names_with_quotes_and_backslashes_round_trip() {
        let mut config = sample();
        config.palette[0].picks[0].ink =
            "Odd \"name\" \\ with ' quotes, tab\t, bell\u{7}, é".into();
        let text = config.to_toml(&[]);
        assert_eq!(PaletteConfig::parse(&text).unwrap(), config);
    }

    #[test]
    fn rejects_syntax_errors_unknown_fields_and_duplicate_sizes() {
        assert!(matches!(
            PaletteConfig::parse("[[palette]\n"),
            Err(ConfigError::Syntax(_))
        ));
        assert!(matches!(
            PaletteConfig::parse("[[palette]]\nsize = 3\npicks = []\nextra = 1\n"),
            Err(ConfigError::Syntax(_))
        ));
        assert_eq!(
            PaletteConfig::parse(
                "[[palette]]\nsize = 3\npicks = []\n[[palette]]\nsize = 3\npicks = []\n"
            ),
            Err(ConfigError::DuplicateSize)
        );
    }

    fn sized(size: u32, inks: &[&str]) -> String {
        let picks: String = inks
            .iter()
            .enumerate()
            .map(|(i, ink)| format!("  {{ rgba = [{}, 0, 0, 255], ink = {ink:?} }},\n", i % 256))
            .collect();
        format!("[[palette]]\nsize = {size}\npicks = [\n{picks}]\n")
    }

    #[test]
    fn rejects_more_distinct_inks_than_the_size() {
        // Review fix F4: size N means up to N distinct inks.
        let err = PaletteConfig::parse(&sized(3, &["A", "B", "C", "D"])).unwrap_err();
        assert_eq!(err, ConfigError::TooManyInks { size: 3, inks: 4 });
        assert!(err.to_string().contains("4 distinct inks"));
        assert!(PaletteConfig::parse(&sized(3, &["A", "B", "C", "A"])).is_ok());
    }

    #[test]
    fn enforces_the_limits() {
        let big = format!("# {}\n", "x".repeat(MAX_BYTES));
        assert!(matches!(
            PaletteConfig::parse(&big),
            Err(ConfigError::TooLarge { .. })
        ));
        let sections: String = (1..=MAX_SECTIONS as u32 + 1)
            .map(|s| format!("[[palette]]\nsize = {s}\npicks = []\n"))
            .collect();
        assert_eq!(
            PaletteConfig::parse(&sections),
            Err(ConfigError::TooManySections {
                count: MAX_SECTIONS + 1
            })
        );
        let picks = vec!["A"; MAX_PICKS + 1];
        assert_eq!(
            PaletteConfig::parse(&sized(1, &picks)),
            Err(ConfigError::TooManyPicks {
                size: 1,
                count: MAX_PICKS + 1
            })
        );
        assert!(PaletteConfig::parse(&sized(1, &vec!["A"; MAX_PICKS])).is_ok());
    }

    #[test]
    fn resolve_composites_over_white_and_finds_the_ink() {
        let resolved = sample().size(3).unwrap().resolve(&palette()).unwrap();
        assert_eq!(resolved[0].matching, Rgb8::new(210, 120, 40));
        assert_eq!(resolved[0].index, 1);
        // A transparent pick is matched as white, like an interactive pick.
        assert_eq!(resolved[1].matching, Rgb8::new(255, 255, 255));
        assert_eq!(resolved[1].rgba, Rgba8::new(0, 0, 0, 0));

        let mappings = sample().size(3).unwrap().mappings(&palette()).unwrap();
        assert_eq!(mappings[0].source, Rgb8::new(210, 120, 40));
        assert_eq!(mappings[0].ink, Rgb8::new(209, 91, 5));
    }

    #[test]
    fn unknown_inks_and_sizes_are_errors() {
        let unknown = SizedPalette {
            size: 1,
            picks: vec![ConfigPick {
                rgba: [0, 0, 0, 255],
                ink: "Nope".into(),
            }],
        };
        assert_eq!(
            unknown.resolve(&palette()),
            Err(ConfigError::UnknownInk("Nope".into()))
        );
        assert_eq!(sample().size(16), Err(ConfigError::MissingSize(16)));
    }
}
