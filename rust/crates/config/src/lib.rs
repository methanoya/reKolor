//! Palette config files: `*.palettes.toml`. Shared by the CLI (golden set) and the web
//! app (import/export through `rekolor-wasm`), so both read and validate files the same way.
//!
//! ```toml
//! material = [0, 0, 0]
//! unprinted = [
//!   { material = true, delta_e = 10 },
//!   { rgba = [200, 40, 40, 255], delta_e = 12.5 },
//! ]
//!
//! [[palette]]
//! size = 3
//! picks = [
//!   { rgba = [210, 120, 40, 255], ink = "Pantone 1595" },
//! ]
//! ```
//!
//! `material` is the color the picks are composited over (the garment or substrate): read as white
//! when absent, always written. `unprinted` lists the colors
//! left unprinted: a stored pixel color and a ΔE, or the material's own color
//! (`material = true`, which follows the material); none when absent, always written. Both apply to
//! every section. Each pick is a pixel color (straight-alpha RGBA, as stored in the image) and the
//! palette entry it maps to, by name. "Size" is the requested number of distinct inks; a palette
//! may list fewer picks ("up to N"), and several picks may share one ink.
//!
//! No file I/O and no dependency on `rekolor-io` or `wasm-bindgen`: callers read and write the
//! text.

// How the file is read: the structs below mirror the TOML layout, and serde (with the `toml` crate)
// fills them from the text. `#[derive(Deserialize)]` generates that reading code;
// `#[serde(...)]` attributes adjust it (defaults, unknown keys, custom conversions).
use std::collections::HashSet;

use rekolor_core::{Mapping, MaterialRange, Palette, Rgb8, Rgba8, composite};
use serde::Deserialize;

// Limits that keep a malicious or broken file from using too much memory or time.
/// Largest accepted config text.
pub const MAX_BYTES: usize = 256 * 1024;
/// Most `[[palette]]` sections in one config.
pub const MAX_SECTIONS: usize = 64;
/// Most picks in one section.
pub const MAX_PICKS: usize = 256;
/// Most unprinted colors in one config (the same limit as picks).
pub const MAX_UNPRINTED: usize = 256;
/// The ΔE a new unprinted color starts with (the app's default too).
pub const DEFAULT_UNPRINTED_DELTA_E: f32 = 10.0;

// The whole file. `deny_unknown_fields` makes a misspelled key an error instead of silently
// ignoring it. `[u8; 3]` is a fixed-size array of three bytes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteConfig {
    // `default = "white"`: when the key is missing, call the function `white()` below.
    /// The material color, `[r, g, b]`; white when the file has no `material` line.
    #[serde(default = "white")]
    pub material: [u8; 3],
    /// Colors left unprinted, for every section; none when the file has no `unprinted` line.
    #[serde(default)]
    pub unprinted: Vec<Unprinted>,
    pub palette: Vec<SizedPalette>,
}

// `try_from = "RawUnprinted"`: serde first reads the simpler struct `RawUnprinted` (exactly as
// written in the file), then converts it with the `TryFrom` implementation below, which rejects
// entries that are neither a color nor the material.
/// A color left unprinted: pixels within `delta_e` (CIEDE2000) of it take no ink. In the file:
/// `{ rgba = [r, g, b, a], delta_e = 10 }` or `{ material = true, delta_e = 10 }`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(try_from = "RawUnprinted")]
pub enum Unprinted {
    /// A stored pixel color from the image (straight-alpha RGBA).
    Color { rgba: [u8; 4], delta_e: f32 },
    /// The material's own color, whatever the material is.
    Material { delta_e: f32 },
}

impl Unprinted {
    // The ΔE of either kind of entry. `match *self` looks inside the enum; `{ delta_e, .. }` takes
    // the `delta_e` field and ignores the rest; `|` handles two variants in one arm.
    pub fn delta_e(&self) -> f32 {
        match *self {
            Unprinted::Color { delta_e, .. } | Unprinted::Material { delta_e } => delta_e,
        }
    }

    /// The engine's range on `material` (the material's own entry becomes that color, opaque).
    pub fn range(&self, material: Rgb8) -> MaterialRange {
        let pixel = match *self {
            Unprinted::Color { rgba, .. } => Rgba8::from(rgba),
            Unprinted::Material { .. } => Rgba8::new(material.r, material.g, material.b, 255),
        };
        MaterialRange {
            pixel,
            delta_e: self.delta_e(),
        }
    }
}

// Private: only used while reading. `rgba` is optional (`Option`), `material` defaults to false.
/// An `unprinted` entry as written: exactly one of `rgba` and `material = true`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUnprinted {
    rgba: Option<[u8; 4]>,
    #[serde(default)]
    material: bool,
    delta_e: f32,
}

// The conversion serde runs after reading a `RawUnprinted`. `type Error = String` says a failed
// conversion is described by a text message (serde turns it into a syntax error for the file).
impl TryFrom<RawUnprinted> for Unprinted {
    type Error = String;

    fn try_from(raw: RawUnprinted) -> Result<Self, Self::Error> {
        // Match both fields at once: exactly one of "a color" or "the material" must be given.
        match (raw.rgba, raw.material) {
            (Some(rgba), false) => Ok(Unprinted::Color {
                rgba,
                delta_e: raw.delta_e,
            }),
            (None, true) => Ok(Unprinted::Material {
                delta_e: raw.delta_e,
            }),
            _ => Err("an unprinted color needs either `rgba` or `material = true`".into()),
        }
    }
}

// The default `material`, used by `#[serde(default = "white")]` above.
fn white() -> [u8; 3] {
    [255, 255, 255]
}

// One `[[palette]]` section of the file: a palette size and its picks. In TOML, `[[name]]` starts
// one entry of a list of tables, so a file can have several sections.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizedPalette {
    pub size: u32,
    pub picks: Vec<ConfigPick>,
}

// One pick as written: the pixel color (RGBA) and the ink's name in the palette file.
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
    /// The color the matcher sees: `rgba` composited over the material (like an interactive pick).
    pub matching: Rgb8,
    /// The ink's position in the palette.
    pub index: usize,
}

// Everything that can be wrong with a config, each with the message shown to the user. In an
// `#[error]` message, `{0}` is the variant's first unnamed field and `{0:?}` prints it quoted.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
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
    #[error("{count} unprinted colors; at most {max} are allowed", max = MAX_UNPRINTED)]
    TooManyUnprinted { count: usize },
    #[error("an unprinted color's delta_e must be from 0 to 100, got {0}")]
    UnprintedDeltaE(f32),
    #[error("the material is listed as unprinted more than once")]
    DuplicateMaterialUnprinted,
}

impl PaletteConfig {
    // `toml::from_str` reads the text into a `PaletteConfig`; a TOML syntax error, a wrong type or
    // an unknown key comes back as an error, which `map_err` wraps as `ConfigError::Syntax`.
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
        if self.unprinted.len() > MAX_UNPRINTED {
            return Err(ConfigError::TooManyUnprinted {
                count: self.unprinted.len(),
            });
        }
        for entry in &self.unprinted {
            let delta_e = entry.delta_e();
            if !(0.0..=100.0).contains(&delta_e) {
                return Err(ConfigError::UnprintedDeltaE(delta_e));
            }
        }
        // At most one entry may stand for the material's own color. `matches!` tests a value
        // against a pattern and gives true or false.
        let materials = self
            .unprinted
            .iter()
            .filter(|u| matches!(u, Unprinted::Material { .. }))
            .count();
        if materials > 1 {
            return Err(ConfigError::DuplicateMaterialUnprinted);
        }
        if self.palette.len() > MAX_SECTIONS {
            return Err(ConfigError::TooManySections {
                count: self.palette.len(),
            });
        }
        // `insert` returns false if the value was already in the set: a size used twice.
        let mut sizes = HashSet::new();
        for palette in &self.palette {
            if !sizes.insert(palette.size) {
                return Err(ConfigError::DuplicateSize);
            }
            palette.validate()?;
        }
        Ok(())
    }

    // The section with the given size, or a `MissingSize` error.
    pub fn size(&self, size: u32) -> Result<&SizedPalette, ConfigError> {
        self.palette
            .iter()
            .find(|p| p.size == size)
            .ok_or(ConfigError::MissingSize(size))
    }

    /// The engine's ranges for the unprinted colors on `material` (the config's, or an override).
    pub fn ranges(&self, material: Rgb8) -> Vec<MaterialRange> {
        self.unprinted.iter().map(|u| u.range(material)).collect()
    }

    // Writes the file by hand rather than with a TOML library, so the layout is always the same
    // (one pick per line), which keeps diffs of the golden configs small and readable. `format!`
    // builds a `String` like `println!` prints; `{r}` inserts the variable `r`, and `{{`/`}}` are
    // literal braces.
    /// Writes the config in the simple format: each header line as a `# ` comment (see
    /// `comment`), the material (white too), the unprinted colors (none too), then the sections in
    /// order.
    pub fn to_toml(&self, header: &[&str]) -> String {
        // Collecting an iterator of `String`s into one `String` concatenates them.
        let mut out: String = header.iter().map(|line| comment(line)).collect();
        let [r, g, b] = self.material;
        out.push_str(&format!("\nmaterial = [{r}, {g}, {b}]\n"));
        if self.unprinted.is_empty() {
            out.push_str("unprinted = []\n");
        } else {
            out.push_str("unprinted = [\n");
            for entry in &self.unprinted {
                // `Display` for f32 writes 10.0 as "10" (a TOML integer, read back as 10.0).
                match *entry {
                    Unprinted::Color { rgba, delta_e } => {
                        let [r, g, b, a] = rgba;
                        out.push_str(&format!(
                            "  {{ rgba = [{r}, {g}, {b}, {a}], delta_e = {delta_e} }},\n"
                        ));
                    }
                    Unprinted::Material { delta_e } => {
                        out.push_str(&format!("  {{ material = true, delta_e = {delta_e} }},\n"));
                    }
                }
            }
            out.push_str("]\n");
        }
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

// A header line can hold text from outside, such as the image's file name in the web app's export.
// A line break in it would end the comment and leave the rest as (invalid) TOML, and TOML allows
// no other control character in a comment except tab.
/// `line` as TOML comment lines: each line break in it (`\n` or `\r\n`) starts another `# ` line,
/// and every other control character except tab becomes U+FFFD (�).
fn comment(line: &str) -> String {
    line.split('\n')
        .map(|part| {
            let text: String = part
                .strip_suffix('\r')
                .unwrap_or(part)
                .chars()
                .map(|c| {
                    if c.is_control() && c != '\t' {
                        '\u{FFFD}'
                    } else {
                        c
                    }
                })
                .collect();
            format!("# {text}\n")
        })
        .collect()
}

// Quotes an ink name for TOML: quotes, backslashes and control characters must be escaped.
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
        // Several picks may share an ink, so count the distinct ink names, not the picks.
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
    /// composites each pick over `material` (the config's, or an override), in pick order.
    pub fn resolve(
        &self,
        palette: &Palette,
        material: Rgb8,
    ) -> Result<Vec<ResolvedPick>, ConfigError> {
        self.picks
            .iter()
            .map(|pick| {
                // Find the ink by name; `ok_or_else(...)?` turns "not found" into an `UnknownInk`
                // error and returns it. Collecting into a `Result<Vec<_>, _>` stops at the first
                // error.
                let index = palette
                    .entries()
                    .iter()
                    .position(|e| e.name == pick.ink)
                    .ok_or_else(|| ConfigError::UnknownInk(pick.ink.clone()))?;
                let rgba = Rgba8::from(pick.rgba);
                Ok(ResolvedPick {
                    rgba,
                    matching: composite(rgba, material),
                    index,
                })
            })
            .collect()
    }

    // `into_iter` (instead of `iter`) consumes the resolved list, taking each item by value.
    /// The mappings for recoloring on `material`: each pick's matching color → its ink's color.
    pub fn mappings(&self, palette: &Palette, material: Rgb8) -> Result<Vec<Mapping>, ConfigError> {
        Ok(self
            .resolve(palette, material)?
            .into_iter()
            .map(|p| Mapping {
                source: p.matching,
                ink: palette.entries()[p.index].rgb,
            })
            .collect())
    }
}

// Unit tests for this file, run with `cargo test -p rekolor-config`.
#[cfg(test)]
mod tests {
    use super::*;
    use rekolor_core::PaletteEntry;

    fn sample() -> PaletteConfig {
        PaletteConfig {
            material: [20, 20, 22],
            unprinted: vec![
                Unprinted::Material { delta_e: 10.0 },
                Unprinted::Color {
                    rgba: [200, 40, 40, 255],
                    delta_e: 12.5,
                },
            ],
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
        assert!(text.starts_with(
            "# Header line one.\n# Two.\n\nmaterial = [20, 20, 22]\nunprinted = [\n\
             \x20 { material = true, delta_e = 10 },\n\
             \x20 { rgba = [200, 40, 40, 255], delta_e = 12.5 },\n]\n\n[[palette]]\nsize = 3\n"
        ));
        assert!(text.contains("  { rgba = [210, 120, 40, 255], ink = \"Pantone 1595\" },\n"));
        assert_eq!(PaletteConfig::parse(&text).unwrap(), sample());
    }

    #[test]
    fn header_lines_stay_comments_whatever_they_contain() {
        // The web app's header includes the image's file name, which may contain line breaks and
        // control characters (TOML allows neither in a comment, except tab).
        let text = sample().to_toml(&["For a\nb\r\nc\rd\u{0}e\u{7f}f\tg.png.", ""]);
        assert!(text.starts_with(
            "# For a\n# b\n# c\u{FFFD}d\u{FFFD}e\u{FFFD}f\tg.png.\n# \n\nmaterial = "
        ));
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
        // Size N means up to N distinct inks.
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
    fn the_material_is_white_when_absent_and_always_written() {
        let config = PaletteConfig::parse("[[palette]]\nsize = 3\npicks = []\n").unwrap();
        assert_eq!(config.material, [255, 255, 255]);
        let text = config.to_toml(&[]);
        assert_eq!(
            text,
            "\nmaterial = [255, 255, 255]\nunprinted = []\n\n[[palette]]\nsize = 3\npicks = [\n]\n"
        );
        assert_eq!(PaletteConfig::parse(&text).unwrap(), config);
    }

    #[test]
    fn rejects_a_malformed_material() {
        for text in [
            "material = [1, 2]\n",
            "material = [1, 2, 3, 4]\n",
            "material = [256, 0, 0]\n",
            "material = [-1, 0, 0]\n",
            "material = \"white\"\n",
            // After a table, the key belongs to that section, which has no such field.
            "[[palette]]\nsize = 3\npicks = []\nmaterial = [0, 0, 0]\n",
        ] {
            assert!(
                matches!(PaletteConfig::parse(text), Err(ConfigError::Syntax(_))),
                "{text:?}"
            );
        }
    }

    #[test]
    fn resolve_composites_over_the_material_and_finds_the_ink() {
        let config = sample();
        let sized = config.size(3).unwrap();
        let resolved = sized.resolve(&palette(), Rgb8::WHITE).unwrap();
        assert_eq!(resolved[0].matching, Rgb8::new(210, 120, 40));
        assert_eq!(resolved[0].index, 1);
        // A transparent pick is matched as the material, like an interactive pick.
        assert_eq!(resolved[1].matching, Rgb8::new(255, 255, 255));
        assert_eq!(resolved[1].rgba, Rgba8::new(0, 0, 0, 0));
        let resolved = sized.resolve(&palette(), config.material.into()).unwrap();
        assert_eq!(resolved[0].matching, Rgb8::new(210, 120, 40));
        assert_eq!(resolved[1].matching, Rgb8::new(20, 20, 22));

        let mappings = sized.mappings(&palette(), config.material.into()).unwrap();
        assert_eq!(mappings[0].source, Rgb8::new(210, 120, 40));
        assert_eq!(mappings[0].ink, Rgb8::new(209, 91, 5));
        assert_eq!(mappings[1].source, Rgb8::new(20, 20, 22));
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
            unknown.resolve(&palette(), Rgb8::WHITE),
            Err(ConfigError::UnknownInk("Nope".into()))
        );
        assert_eq!(sample().size(16), Err(ConfigError::MissingSize(16)));
    }

    #[test]
    fn unprinted_colors_are_read_and_turned_into_ranges() {
        // Absent: none.
        let none = PaletteConfig::parse("[[palette]]\nsize = 1\npicks = []\n").unwrap();
        assert!(none.unprinted.is_empty());
        // Both kinds; an integer delta_e reads as a float.
        let text = "material = [0, 0, 0]\nunprinted = [\n\
                    { material = true, delta_e = 10 },\n\
                    { rgba = [9, 9, 9, 0], delta_e = 2.5 },\n]\n\n\
                    [[palette]]\nsize = 1\npicks = []\n";
        let config = PaletteConfig::parse(text).unwrap();
        assert_eq!(
            config.unprinted,
            [
                Unprinted::Material { delta_e: 10.0 },
                Unprinted::Color {
                    rgba: [9, 9, 9, 0],
                    delta_e: 2.5
                }
            ]
        );
        // The material's entry is the given material (the config's, or a CLI override), opaque.
        let blue = Rgb8::new(20, 140, 230);
        assert_eq!(
            config.ranges(blue),
            [
                MaterialRange {
                    pixel: Rgba8::new(20, 140, 230, 255),
                    delta_e: 10.0
                },
                MaterialRange {
                    pixel: Rgba8::new(9, 9, 9, 0),
                    delta_e: 2.5
                }
            ]
        );
    }

    #[test]
    fn rejects_malformed_unprinted_colors() {
        let with = |entries: &str| {
            PaletteConfig::parse(&format!(
                "unprinted = [{entries}]\n\n[[palette]]\nsize = 1\npicks = []\n"
            ))
        };
        for shape in [
            "{ delta_e = 1 }",
            "{ rgba = [1, 2, 3, 4], material = true, delta_e = 1 }",
            "{ material = false, delta_e = 1 }",
            "{ rgba = [1, 2, 3], delta_e = 1 }",
            "{ material = true }",
            "{ material = true, delta_e = 1, extra = 1 }",
        ] {
            assert!(
                matches!(with(shape), Err(ConfigError::Syntax(_))),
                "{shape}"
            );
        }
        assert_eq!(
            with("{ material = true, delta_e = 100.5 }"),
            Err(ConfigError::UnprintedDeltaE(100.5))
        );
        assert!(matches!(
            with("{ material = true, delta_e = nan }"),
            Err(ConfigError::UnprintedDeltaE(_))
        ));
        assert_eq!(
            with("{ material = true, delta_e = 1 }, { material = true, delta_e = 2 }"),
            Err(ConfigError::DuplicateMaterialUnprinted)
        );
        let many = vec!["{ rgba = [1, 2, 3, 4], delta_e = 1 }"; MAX_UNPRINTED + 1].join(", ");
        assert_eq!(
            with(&many),
            Err(ConfigError::TooManyUnprinted {
                count: MAX_UNPRINTED + 1
            })
        );
    }
}
