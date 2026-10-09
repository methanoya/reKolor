//! The Pantone suggestion must match the snapshot in `testdata/baseline/pantone-suggestions.tsv`
//! exactly (the baseline is current approved behavior; update it with
//! `cargo run --release -p rekolor-core --example update_baseline` and review the diff).

// An integration test (see `fingerprint.rs` for how these files work). The snapshot lists, for
// each of 5,096 generated colors, the Pantone ink the engine suggests and its ΔE (6 decimals).
#[path = "../../../testdata/generator.rs"]
mod generator;

use std::path::Path;

use rekolor_core::{Palette, PaletteEntry, Rgb8};

// `concat!` joins string literals at compile time. `&'static` means the result borrows data that
// lives for the whole program (the literal), so it can be returned freely.
fn repo_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."))
}

/// `palettes/pantone.json`, in file order (serde_json's `preserve_order`).
fn pantone() -> Palette {
    let text = std::fs::read_to_string(repo_root().join("palettes/pantone.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let entries = json
        .as_object()
        .expect("palette is a JSON object")
        .iter()
        .map(|(name, value)| {
            let rgb: Vec<u8> = value["rgb"]
                .as_array()
                .expect("rgb array")
                .iter()
                .map(|v| v.as_u64().expect("channel") as u8)
                .collect();
            PaletteEntry::new(name.clone(), Rgb8::new(rgb[0], rgb[1], rgb[2]))
        })
        .collect();
    Palette::new(entries).unwrap()
}

// The snapshot rows: (color, ink name, ΔE as written).
fn snapshot() -> Vec<([u8; 3], String, String)> {
    std::fs::read_to_string(repo_root().join("rust/testdata/baseline/pantone-suggestions.tsv"))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            let c: Vec<u8> = cols[..3].iter().map(|v| v.parse().unwrap()).collect();
            ([c[0], c[1], c[2]], cols[3].to_string(), cols[4].to_string())
        })
        .collect()
}

#[test]
fn snapshot_covers_the_generated_colors() {
    let colors: Vec<[u8; 3]> = snapshot().into_iter().map(|(c, _, _)| c).collect();
    let expected = generator::suggestion_colors();
    assert_eq!(expected.len(), 5096);
    let distinct: std::collections::HashSet<_> = expected.iter().collect();
    assert_eq!(
        distinct.len(),
        expected.len(),
        "suggestion colors must be distinct"
    );
    assert_eq!(
        colors, expected,
        "snapshot colors differ from generator::suggestion_colors()"
    );
}

#[test]
fn suggestions_match_the_snapshot() {
    let palette = pantone();
    assert_eq!(palette.entries().len(), 909);
    let mut failures = Vec::new();
    for ([r, g, b], name, delta_e) in snapshot() {
        let m = palette.suggest(Rgb8::new(r, g, b));
        // Compare with the same 6-decimal formatting the file uses (`{:.6}`), so tiny float noise
        // in the last bits can't cause false failures, while any real change still shows.
        let actual = (m.entry.name.as_str(), format!("{:.6}", m.delta_e));
        if actual != (name.as_str(), delta_e.clone()) {
            failures.push(format!(
                "[{r}, {g}, {b}]: snapshot {name} ({delta_e}), now {} ({})",
                actual.0, actual.1
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of 5096 suggestions differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
