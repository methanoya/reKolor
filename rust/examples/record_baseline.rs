//! Records the R9 baseline: runs every generated fixture through the existing implementation
//! (`image_info` and `replace_rgb_colors`) and writes the results to `testdata/baseline/`.
//!
//!     cargo run --release --example record_baseline [-- --inputs <dir>]
//!
//! `--inputs <dir>` additionally writes each fixture as a lossless PNG to `<dir>`, so the same
//! inputs can be fed to the WASM build for a cross-check.
//!
//! The old modules are included unchanged, the same way `src/main.rs` does it.

#![allow(dead_code)]

#[path = "../src/console.rs"]
mod console;
#[path = "../src/conv.rs"]
mod conv;
#[path = "../testdata/generator.rs"]
mod generator;
#[path = "../src/info.rs"]
mod info;
#[path = "../src/utils.rs"]
mod utils;

use std::fmt::Write as _;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use conv::{RgbColor8, RgbColorReplacementPair};
use image::{ImageFormat, RgbaImage};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let inputs_dir = args
        .iter()
        .position(|a| a == "--inputs")
        .map(|i| PathBuf::from(args.get(i + 1).expect("--inputs needs a directory")));

    let baseline = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/baseline");
    let recolor_dir = baseline.join("recolor");
    std::fs::create_dir_all(&recolor_dir).expect("create testdata/baseline/recolor");
    if let Some(dir) = &inputs_dir {
        std::fs::create_dir_all(dir).expect("create inputs dir");
    }

    let mut info_tsv = String::from("# fixture\twidth\theight\trgb_colors\treal_colors\n");
    for fixture in generator::fixtures() {
        let png = encode_png(&fixture);
        // The PNG round trip must be lossless, otherwise the baseline would not describe the fixture.
        let decoded = image::load_from_memory(&png)
            .expect("decode fixture")
            .to_rgba8();
        assert_eq!(
            decoded.as_raw(),
            &fixture.rgba,
            "lossless PNG round trip: {}",
            fixture.name
        );
        if let Some(dir) = &inputs_dir {
            std::fs::write(dir.join(format!("{}.png", fixture.name)), &png).expect("write input");
        }

        let i = info::image_info(&png).expect("image_info");
        writeln!(
            info_tsv,
            "{}\t{}\t{}\t{}\t{}",
            fixture.name, i.width, i.height, i.rgb_colors, i.real_colors
        )
        .unwrap();

        for set in generator::MAPPING_SETS {
            let pairs: Vec<RgbColorReplacementPair> = set
                .pairs
                .iter()
                .map(|&(from, to)| RgbColorReplacementPair {
                    from: rgb(from),
                    to: rgb(to),
                })
                .collect();
            let result = conv::replace_rgb_colors(&png, &pairs).expect("replace_rgb_colors");
            assert_eq!(
                (result.width, result.height),
                (fixture.width, fixture.height),
                "output size: {} × {}",
                fixture.name,
                set.name
            );
            // Stored exactly as returned (a PNG); tests compare decoded pixels, never bytes.
            let path = recolor_dir.join(format!("{}__{}.png", fixture.name, set.name));
            std::fs::write(&path, &result.image).expect("write baseline output");
        }
    }
    std::fs::write(baseline.join("image-info.tsv"), info_tsv).expect("write image-info.tsv");
    eprintln!("baseline written to {}", baseline.display());
}

fn rgb([r, g, b]: [u8; 3]) -> RgbColor8 {
    RgbColor8 { r, g, b }
}

fn encode_png(fixture: &generator::Fixture) -> Vec<u8> {
    let img = RgbaImage::from_raw(fixture.width, fixture.height, fixture.rgba.clone())
        .expect("fixture buffer size");
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .expect("encode fixture");
    png
}
