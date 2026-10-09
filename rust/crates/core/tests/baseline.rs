//! `core` must match the baseline snapshot in `testdata/baseline/` pixel for pixel (the current
//! approved behavior). Inputs come from `testdata/generator.rs`. After an intentional change:
//! `cargo run --release -p rekolor-core --example update_baseline`, review the diff, commit.

// An integration test (see `fingerprint.rs` for how these files work). The test images are
// generated in code by `generator.rs`, and the expected outputs are PNG files in
// `testdata/baseline/recolor/`, named `<image>__<mapping set>.png`.
#[path = "../../../testdata/generator.rs"]
mod generator;

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use rekolor_core::{ImageRef, Mapping, Rgb8, analyze, recolor};

// `env!("CARGO_MANIFEST_DIR")` is this crate's folder, filled in at compile time, so the tests
// find their files whatever the current directory is.
fn baseline_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/baseline")
}

/// Decodes a baseline PNG to RGBA8: (width, height, pixels).
fn decode_png(path: &Path) -> (u32, u32, Vec<u8>) {
    // In tests, `unwrap`/`expect` are fine: a failure panics, and a panic fails the test with its
    // message.
    let file = File::open(path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("PNG header");
    let mut buf = vec![0; reader.output_buffer_size().expect("PNG buffer size")];
    let info = reader.next_frame(&mut buf).expect("PNG frame");
    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "{} is not RGBA",
        path.display()
    );
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

/// Pixel comparison with a readable report: the number of differing pixels and the first
/// few with expected/actual colors.
fn compare_pixels(label: &str, width: u32, expected: &[u8], actual: &[u8]) -> Result<(), String> {
    assert_eq!(expected.len(), actual.len(), "{label}: buffer length");
    // Indexes of the pixels that differ; `a[i..j]` is the slice of bytes from `i` up to (not
    // including) `j`, here one pixel's four bytes.
    let differing: Vec<usize> = (0..expected.len() / 4)
        .filter(|i| expected[i * 4..i * 4 + 4] != actual[i * 4..i * 4 + 4])
        .collect();
    if differing.is_empty() {
        return Ok(());
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
    Err(format!(
        "{label}: {} of {} pixels differ; first: {}",
        differing.len(),
        expected.len() / 4,
        first.join("; ")
    ))
}

// Every fixture image × every mapping set: collect all mismatches first and fail once at the
// end, so a single run reports every difference.
#[test]
fn recolor_reproduces_the_baseline() {
    let mut failures = Vec::new();
    let mut cases = 0;
    for fixture in generator::fixtures() {
        let image = ImageRef::new(&fixture.rgba, fixture.width, fixture.height).unwrap();
        for set in generator::MAPPING_SETS {
            let mappings: Vec<Mapping> = set
                .pairs
                .iter()
                .map(|&(source, ink)| Mapping {
                    source: source.into(),
                    ink: ink.into(),
                })
                .collect();
            let mut out = vec![0; fixture.rgba.len()];
            let stats = recolor(image, &mappings, Rgb8::WHITE, &mut out).unwrap();
            assert_eq!(
                stats.exact + stats.nearest,
                u64::from(fixture.width * fixture.height)
            );

            let label = format!("{} × {}", fixture.name, set.name);
            let path = baseline_dir().join(format!("recolor/{}__{}.png", fixture.name, set.name));
            let (w, h, expected) = decode_png(&path);
            assert_eq!((w, h), (fixture.width, fixture.height), "{label}: size");
            if let Err(e) = compare_pixels(&label, w, &expected, &out) {
                failures.push(e);
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 30);
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

// `image-info.tsv` holds one tab-separated row per fixture: name, width, height, colors and RGBA
// colors. Lines starting with `#` are comments.
#[test]
fn analyze_reproduces_image_info() {
    let tsv = std::fs::read_to_string(baseline_dir().join("image-info.tsv")).unwrap();
    let rows: Vec<Vec<&str>> = tsv
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').collect())
        .collect();
    assert_eq!(rows.len(), generator::fixtures().len());
    for row in rows {
        let fixture = generator::fixture(row[0]);
        let image = ImageRef::new(&fixture.rgba, fixture.width, fixture.height).unwrap();
        let stats = analyze(image, Rgb8::WHITE);
        let expected: Vec<u64> = row[1..].iter().map(|v| v.parse().unwrap()).collect();
        assert_eq!(
            vec![
                u64::from(stats.width),
                u64::from(stats.height),
                stats.colors,
                stats.rgba_colors
            ],
            expected,
            "{}: width, height, colors, rgba_colors",
            row[0]
        );
    }
}
