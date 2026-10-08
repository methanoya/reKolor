//! Rewrites the baseline snapshot in `rust/testdata/baseline/` from the current `rekolor-core`
//! (P1: the baseline is "current approved behavior"; review the diff, then commit).
//!
//!     cargo run -p rekolor-core --example update_baseline [-- --colors-from-current]
//!
//! - `recolor/<fixture>__<set>.png`: rewritten only when the decoded pixels differ, so an unchanged
//!   engine leaves these files byte-identical.
//! - `image-info.tsv`: `analyze` for every fixture.
//! - `pantone-suggestions.tsv`: `Palette::suggest` for `generator::suggestion_colors()`, or with
//!   `--colors-from-current` for the colors already in the file (e.g. to compare two ΔE
//!   implementations on the same colors).
//!
//! Tests only read these files. The 2023 JavaScript suggestions are in git history (`c0d094c`).

#[path = "../../../testdata/generator.rs"]
mod generator;

use std::fmt::Write as _;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use rekolor_core::{ImageRef, Mapping, Palette, PaletteEntry, Rgb8, analyze, recolor};

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
                })
                .collect();
            let mut out = vec![0; fixture.rgba.len()];
            recolor(image, &mappings, &mut out).unwrap();
            let path = baseline.join(format!("recolor/{}__{}.png", fixture.name, set.name));
            if decode_png(&path).as_deref() != Some(&out[..]) {
                write_png(&path, &out, fixture.width, fixture.height);
                changed.push(relative(&path, &rust_dir));
            }
        }
    }

    // Image info.
    let mut info = String::from("# fixture\twidth\theight\tcolors\trgba_colors\n");
    for fixture in generator::fixtures() {
        let s = analyze(ImageRef::new(&fixture.rgba, fixture.width, fixture.height).unwrap());
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

/// RGBA8 pixels of a PNG, or `None` if the file is missing or unreadable.
fn decode_png(path: &Path) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path).ok()?));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    (info.color_type == png::ColorType::Rgba).then(|| {
        buf.truncate(info.buffer_size());
        buf
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
    if std::fs::read_to_string(path).ok().as_deref() != Some(content) {
        std::fs::write(path, content).unwrap();
        changed.push(relative(path, root));
    }
}

fn relative(path: &Path, root: &Path) -> String {
    let (path, root) = (path.canonicalize().unwrap(), root.canonicalize().unwrap());
    path.strip_prefix(&root)
        .unwrap_or(&path)
        .display()
        .to_string()
}
