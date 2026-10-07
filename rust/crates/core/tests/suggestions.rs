//! R10: the Rust Pantone suggestion must match the 2023 JavaScript one (`palette.ts` with
//! `color-diff`, recorded in `testdata/baseline/pantone-suggestions.tsv`), except at near-ties:
//! the two use different CIEDE2000 implementations, so when two entries are almost equally
//! close, they may pick different ones.

use std::path::Path;

use rekolor_core::{Palette, PaletteEntry, Rgb8, delta_e_2000};

/// When Rust and JS suggest different entries, both must be this close to equally near
/// (measured with the Rust ΔE).
const NEAR_TIE_TOLERANCE: f32 = 0.01;

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

#[test]
fn suggestions_match_2023_javascript_except_near_ties() {
    let palette = pantone();
    assert_eq!(palette.entries().len(), 909);
    let tsv =
        std::fs::read_to_string(repo_root().join("rust/testdata/baseline/pantone-suggestions.tsv"))
            .unwrap();

    let (mut total, mut near_ties, mut failures) = (0, Vec::new(), Vec::new());
    for line in tsv.lines().filter(|l| !l.starts_with('#')) {
        let cols: Vec<&str> = line.split('\t').collect();
        let color = Rgb8::new(
            cols[0].parse().unwrap(),
            cols[1].parse().unwrap(),
            cols[2].parse().unwrap(),
        );
        let js_name = cols[3];
        total += 1;

        let rust = palette.suggest(color);
        if rust.entry.name == js_name {
            continue;
        }
        let js_entry = palette
            .entries()
            .iter()
            .find(|e| e.name == js_name)
            .unwrap_or_else(|| panic!("JS suggested unknown entry {js_name}"));
        let gap = (delta_e_2000(color, js_entry.rgb) - rust.delta_e).abs();
        let report = format!(
            "{color:?}: JS {js_name}, Rust {} (ΔE gap {gap:.4})",
            rust.entry.name
        );
        if gap <= NEAR_TIE_TOLERANCE {
            near_ties.push(report);
        } else {
            failures.push(report);
        }
    }
    assert_eq!(total, 5096);
    eprintln!(
        "{} of {total} suggestions identical; {} near-ties: {:?}",
        total - near_ties.len() - failures.len(),
        near_ties.len(),
        near_ties
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
