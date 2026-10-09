//! Rewrites the baseline snapshot in `rust/testdata/baseline/` from the current `rekolor-core`
//! (the baseline is "current approved behavior"; review the diff, then commit).
//!
//!     cargo run --release -p rekolor-core --example update_baseline [-- --colors-from-current]
//!
//! - `recolor/<fixture>__<set>.png`: rewritten only when the decoded pixels differ, so an unchanged
//!   engine leaves these files byte-identical.
//! - `image-info.tsv`: `analyze` for every fixture.
//! - `pantone-suggestions.tsv`: `Palette::suggest` for `generator::suggestion_colors()`, or with
//!   `--colors-from-current` for the colors already in the file (e.g. to compare two ΔE
//!   implementations on the same colors).
//! - `delta-e-fingerprint.tsv`: `generator::delta_e_fingerprint` with `delta_e_2000`; the
//!   native and WASM tests must reproduce it bit for bit.
//!
//! Tests only read these files.

// Files in a crate's `examples/` folder are small programs built against the crate; this one is
// a maintenance tool rather than an example. `#[path]` shares the test-data generator with the
// tests (see `tests/fingerprint.rs`).
#[path = "../../../testdata/generator.rs"]
mod generator;

// Importing the `Write` trait (`as _`: without a name) enables `writeln!(string, ...)`, which
// appends a formatted line to a `String`.
use std::fmt::Write as _;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use rekolor_core::{
    ImageRef, Mapping, Palette, PaletteEntry, Rgb8, analyze, delta_e_2000, recolor,
};

// `--colors-from-current` is checked by hand; a tool this small needs no argument parser.
fn main() {
    let colors_from_current = std::env::args().any(|a| a == "--colors-from-current");
    let rust_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baseline = rust_dir.join("testdata/baseline");
    let mut changed: Vec<String> = Vec::new();

    // Recolor outputs.
    for fixture in generator::fixtures() {
        let image = ImageRef::new(&fixture.rgba, fixture.width, fixture.height).unwrap();
        for set in generator::MAPPING_SETS {
            let mappings: Vec<Mapping> = set
                .pairs
                .iter()
                .map(|&(source, ink)| Mapping {
                    source: source.into(),
                    ink: ink.into(),
                    delta_e: 0.0,
                })
                .collect();
            let mut out = vec![0; fixture.rgba.len()];
            recolor(image, &mappings, Rgb8::WHITE, &mut out).unwrap();
            let path = baseline.join(format!("recolor/{}__{}.png", fixture.name, set.name));
            // Rewrite unless size and pixels both match (the baseline test checks both).
            let current = decode_png(&path);
            // `rgba[..] == out[..]` compares the two buffers byte by byte.
            let unchanged = current.as_ref().is_some_and(|(w, h, rgba)| {
                (*w, *h) == (fixture.width, fixture.height) && rgba[..] == out[..]
            });
            if !unchanged {
                write_png(&path, &out, fixture.width, fixture.height);
                changed.push(relative(&path, &rust_dir));
            }
        }
    }

    // Image info.
    let mut info = String::from("# fixture\twidth\theight\tcolors\trgba_colors\n");
    for fixture in generator::fixtures() {
        let image = ImageRef::new(&fixture.rgba, fixture.width, fixture.height).unwrap();
        let s = analyze(image, Rgb8::WHITE);
        writeln!(
            info,
            "{}\t{}\t{}\t{}\t{}",
            fixture.name, s.width, s.height, s.colors, s.rgba_colors
        )
        .unwrap();
    }
    write_if_changed(
        &baseline.join("image-info.tsv"),
        &info,
        &rust_dir,
        &mut changed,
    );

    // Pantone suggestions.
    let suggestions_path = baseline.join("pantone-suggestions.tsv");
    // `if` is an expression in Rust: each branch produces a value, and the chosen one is stored.
    let colors: Vec<[u8; 3]> = if colors_from_current {
        std::fs::read_to_string(&suggestions_path)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let c: Vec<u8> = l.split('\t').take(3).map(|v| v.parse().unwrap()).collect();
                [c[0], c[1], c[2]]
            })
            .collect()
    } else {
        generator::suggestion_colors()
    };
    let palette = pantone(&rust_dir.join("../palettes/pantone.json"));
    let mut tsv = String::from("# r\tg\tb\tsuggestion\tdelta_e\n");
    for [r, g, b] in colors {
        let m = palette.suggest(Rgb8::new(r, g, b));
        writeln!(tsv, "{r}\t{g}\t{b}\t{}\t{:.6}", m.entry.name, m.delta_e).unwrap();
    }
    write_if_changed(&suggestions_path, &tsv, &rust_dir, &mut changed);

    // ΔE fingerprint: proves native and WASM compute bit-identical ΔE.
    write_if_changed(
        &baseline.join("delta-e-fingerprint.tsv"),
        &generator::delta_e_fingerprint(|x, y| delta_e_2000(x.into(), y.into())),
        &rust_dir,
        &mut changed,
    );

    if changed.is_empty() {
        println!("baseline unchanged");
    } else {
        println!(
            "changed ({}): review with `git diff`, then commit",
            changed.len()
        );
        for path in changed {
            println!("  {path}");
        }
    }
}

fn pantone(path: &Path) -> Palette {
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let entries = json
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, v)| {
            let c: Vec<u8> = v["rgb"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u8)
                .collect();
            PaletteEntry::new(name.clone(), Rgb8::new(c[0], c[1], c[2]))
        })
        .collect();
    Palette::new(entries).unwrap()
}

// `?` also works on `Option`: `.ok()?` turns an error into `None` and returns it.
/// A PNG as RGBA8: (width, height, pixels), or `None` if the file is missing or unreadable.
fn decode_png(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path).ok()?));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    (info.color_type == png::ColorType::Rgba).then(|| {
        buf.truncate(info.buffer_size());
        (info.width, info.height, buf)
    })
}

fn write_png(path: &Path, rgba: &[u8], width: u32, height: u32) {
    let mut encoder = png::Encoder::new(File::create(path).unwrap(), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(rgba)
        .unwrap();
}

fn write_if_changed(path: &Path, content: &str, root: &Path, changed: &mut Vec<String>) {
    // Rewrite only when the text differs, so unchanged files keep their timestamps and `git diff`
    // stays empty.
    if std::fs::read_to_string(path).ok().as_deref() != Some(content) {
        std::fs::write(path, content).unwrap();
        changed.push(relative(path, root));
    }
}

// The path relative to `rust/`, for the summary printed at the end.
fn relative(path: &Path, root: &Path) -> String {
    let (path, root) = (path.canonicalize().unwrap(), root.canonicalize().unwrap());
    path.strip_prefix(&root)
        .unwrap_or(&path)
        .display()
        .to_string()
}
