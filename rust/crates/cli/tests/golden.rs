//! Golden set (X1, X2): every image in `samples/**` recolored from its `<name>.palettes.toml`
//! must match its reviewed `<name>-out-<size>.png` pixel for pixel.
//!
//! Opt-in (slow in debug builds): `cargo test --release -p rekolor-cli --test golden -- --ignored`
//!
//! The test only reads. After an intentional behavior change, regenerate the outputs with
//! `rekolor golden update samples`, review them, then commit. On a mismatch, a diff image
//! (differing pixels in magenta over a faded copy of the expected image) is written to
//! `<temp dir>/rekolor-golden-diff/`.

use std::path::{Path, PathBuf};

use rekolor_cli::config::{self, PaletteConfig};
use rekolor_cli::{discover, palette_file};
use rekolor_core::recolor;

fn repo_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."))
}

/// Compares two RGBA buffers; on a mismatch returns a report and writes a diff image.
fn compare(label: &str, width: u32, height: u32, expected: &[u8], actual: &[u8]) -> Option<String> {
    let differing: Vec<usize> = (0..expected.len() / 4)
        .filter(|&i| expected[i * 4..i * 4 + 4] != actual[i * 4..i * 4 + 4])
        .collect();
    if differing.is_empty() {
        return None;
    }
    let first: Vec<String> = differing
        .iter()
        .take(5)
        .map(|&i| {
            format!(
                "({}, {}) expected {:?} got {:?}",
                i as u32 % width,
                i as u32 / width,
                &expected[i * 4..i * 4 + 4],
                &actual[i * 4..i * 4 + 4]
            )
        })
        .collect();

    // Diff image: faded expected pixels, differing pixels in magenta.
    let mut diff: Vec<u8> = expected
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let fade = |c: u8| ((u16::from(c) + 3 * 255) / 4) as u8;
            [fade(p[0]), fade(p[1]), fade(p[2]), 255]
        })
        .collect();
    for &i in &differing {
        diff[i * 4..i * 4 + 4].copy_from_slice(&[255, 0, 255, 255]);
    }
    let dir = std::env::temp_dir().join("rekolor-golden-diff");
    std::fs::create_dir_all(&dir).expect("create diff dir");
    let diff_path = dir.join(format!("{}.diff.png", label.replace(['/', ' '], "_")));
    rekolor_io::write_png(&diff_path, &diff, width, height).expect("write diff image");

    Some(format!(
        "{label}: {} of {} pixels differ; first: {}; diff image: {}",
        differing.len(),
        expected.len() / 4,
        first.join("; "),
        diff_path.display()
    ))
}

#[test]
#[ignore = "golden set: run with `cargo test --release -p rekolor-cli --test golden -- --ignored`"]
fn every_sample_matches_its_reviewed_golden_outputs() {
    let root = repo_root();
    let samples = root.join("samples");
    let palette = palette_file::load(&root.join(palette_file::DEFAULT_PALETTE)).unwrap();

    let mut checked = 0;
    let mut failures = Vec::new();
    for input in discover::images(&samples).unwrap() {
        let name = input.strip_prefix(&samples).unwrap().display().to_string();
        let config_path = config::config_path(&input);
        let config = match PaletteConfig::load(&config_path) {
            Ok(config) => config,
            Err(e) => {
                failures.push(format!("{name}: {e:#}"));
                continue;
            }
        };
        let image = rekolor_io::decode_file(&input).unwrap();
        for sized in &config.palette {
            let label = format!("{name} size {}", sized.size);
            let golden_path = config::output_path(&input, sized.size);
            let golden = match rekolor_io::decode_file(&golden_path) {
                Ok(golden) => golden,
                Err(e) => {
                    failures.push(format!("{label}: {}: {e}", rel(&golden_path, &root)));
                    continue;
                }
            };
            if (golden.width, golden.height) != (image.width, image.height) {
                failures.push(format!(
                    "{label}: golden is {}×{}, image is {}×{}",
                    golden.width, golden.height, image.width, image.height
                ));
                continue;
            }
            let mappings = sized.mappings(&palette).unwrap();
            let mut out = vec![0; image.rgba.len()];
            recolor(image.view(), &mappings, &mut out).unwrap();
            if let Some(report) = compare(&label, image.width, image.height, &golden.rgba, &out) {
                failures.push(report);
            }
            checked += 1;
        }
    }
    assert!(checked >= 57, "only {checked} golden outputs checked");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

fn rel(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
