//! Loading the palette data file (`palettes/pantone.json`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rekolor_core::{Palette, PaletteEntry, Rgb8};

/// The default palette path, relative to the repository root.
pub const DEFAULT_PALETTE: &str = "palettes/pantone.json";

// `ancestors()` yields the folder itself, then its parent, and so on up to the root. `bail!`
// returns an error with a formatted message.
/// Finds `palettes/pantone.json` in the current directory or the nearest parent that has it,
/// so the default works from the repository root and from `rust/` alike.
pub fn find_default(start: &Path) -> Result<PathBuf> {
    for dir in start.ancestors() {
        let candidate = dir.join(DEFAULT_PALETTE);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!(
        "{DEFAULT_PALETTE} not found in {} or any parent directory; pass --palette <path>",
        start.display()
    )
}

/// Loads a palette file: a JSON object `{ "<name>": { "rgb": [r, g, b] }, … }`.
/// Entries keep the file order, which decides ties between equal colors.
pub fn load(path: &Path) -> Result<Palette> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&text).with_context(|| format!("parsing {}", path.display()))
}

// Parses the JSON text. `serde_json::Value` is "any JSON value"; the `preserve_order` feature (see
// `Cargo.toml`) keeps the object's keys in file order.
pub fn parse(json: &str) -> Result<Palette> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    // `let ... else`: if the value isn't a JSON object, run the `else` block (which must return).
    let Some(object) = value.as_object() else {
        bail!("the palette must be a JSON object of name → {{ \"rgb\": [r, g, b] }}");
    };
    let mut entries = Vec::with_capacity(object.len());
    for (name, entry) in object {
        // Reading into `[u8; 3]` checks the shape for free: exactly three numbers, each 0–255.
        let rgb: [u8; 3] = serde_json::from_value(entry["rgb"].clone())
            .with_context(|| format!("entry {name:?}: \"rgb\" must be three numbers 0–255"))?;
        entries.push(PaletteEntry::new(name.clone(), Rgb8::from(rgb)));
    }
    Ok(Palette::new(entries)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_file_order_and_derives_non_palette() {
        let palette = parse(
            r#"{"Pure White (non-palette)": {"rgb": [255,255,255]}, "B": {"rgb": [1,2,3]}, "A": {"rgb": [1,2,3]}}"#,
        )
        .unwrap();
        let names: Vec<_> = palette.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Pure White (non-palette)", "B", "A"]);
        assert!(palette.entries()[0].non_palette);
    }

    #[test]
    fn rejects_malformed_palettes() {
        assert!(parse("[]").is_err());
        assert!(parse(r#"{"X": {"rgb": [1, 2]}}"#).is_err());
        assert!(parse(r#"{"X": {"rgb": [1, 2, 300]}}"#).is_err());
        assert!(parse("{}").is_err()); // empty palette
    }

    #[test]
    fn finds_the_default_in_a_parent_directory() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("palettes")).unwrap();
        std::fs::write(root.path().join(DEFAULT_PALETTE), "{}").unwrap();
        let nested = root.path().join("rust/crates");
        std::fs::create_dir_all(&nested).unwrap();
        assert_eq!(
            find_default(&nested).unwrap(),
            root.path().join(DEFAULT_PALETTE)
        );
        assert!(find_default(Path::new("/")).is_err() || Path::new("/palettes").exists());
    }
}
