//! Writes the cross-decoder fixtures to `rust/testdata/decoders/`: encoded images with known
//! content, and how `rekolor-io` decodes them (the reference the browser decoder is compared
//! with in `typescript/tests/browser/decoders.test.ts`).
//!
//!     cargo run --release -p rekolor-io --example update_decoder_fixtures
//!
//! For each fixture `<name>.<ext>`: `<name>.rgba` holds the decoded straight-alpha RGBA bytes, and
//! `references.tsv` lists name, file, width, height and the decoder warnings. Everything is
//! generated deterministically, so an unchanged encoder and decoder rewrite identical files. Tests
//! only read these files (`crates/io/tests/decoders.rs` checks they match a fresh decode).

use std::fmt::Write as _;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata/decoders");
    std::fs::create_dir_all(&dir).unwrap();
    let mut tsv = String::from("# name\tfile\twidth\theight\twarnings\n");
    let mut changed = Vec::new();
    for (name, file, bytes) in fixtures() {
        write_if_changed(&dir.join(&file), &bytes, &mut changed);
        let decoded = rekolor_io::decode(&bytes).expect("fixtures decode");
        write_if_changed(
            &dir.join(format!("{name}.rgba")),
            decoded.rgba(),
            &mut changed,
        );
        let warnings: Vec<String> = decoded
            .warnings()
            .iter()
            .map(|w| format!("{w:?}"))
            .collect();
        writeln!(
            tsv,
            "{name}\t{file}\t{}\t{}\t{}",
            decoded.width(),
            decoded.height(),
            warnings.join("; ")
        )
        .unwrap();
    }
    write_if_changed(&dir.join("references.tsv"), tsv.as_bytes(), &mut changed);
    if changed.is_empty() {
        println!("decoder fixtures unchanged");
    } else {
        println!("changed ({}): review, then commit", changed.len());
        for f in changed {
            println!("  {f}");
        }
    }
}

fn write_if_changed(path: &Path, bytes: &[u8], changed: &mut Vec<String>) {
    if std::fs::read(path).ok().as_deref() != Some(bytes) {
        std::fs::write(path, bytes).unwrap();
        changed.push(path.file_name().unwrap().to_string_lossy().into_owned());
    }
}

/// (name, file name, encoded bytes)
fn fixtures() -> Vec<(&'static str, String, Vec<u8>)> {
    vec![
        // Opaque sRGB, no profile: every decoder must agree exactly.
        (
            "opaque",
            "opaque.png".into(),
            png(&gradient(48, 32), 48, 32, None, None),
        ),
        // EXIF orientation 6 (rotate 90° clockwise to display): 24×16 stored, 16×24 shown.
        (
            "exif-6-png",
            "exif-6.png".into(),
            png(&quadrants(24, 16), 24, 16, None, Some(exif_orientation(6))),
        ),
        // The same in a JPEG (lossy: decoders differ slightly, so flat color blocks).
        (
            "exif-6-jpeg",
            "exif-6.jpg".into(),
            jpeg(&blocks(64, 32), 64, 32, exif_orientation(6)),
        ),
        // Every alpha value 0–255 (x) on 8 colors (y).
        (
            "alpha-ramp",
            "alpha-ramp.png".into(),
            png(&alpha_ramp(), 256, 8, None, None),
        ),
        // A valid ICC profile with red and green primaries swapped: browsers apply it, `rekolor-io`
        // reports it and keeps the stored values. Expected to differ.
        (
            "icc-swapped",
            "icc-swapped.png".into(),
            png(&gradient(32, 32), 32, 32, Some(swapped_profile()), None),
        ),
    ]
}

fn png(rgba: &[u8], w: u32, h: u32, icc: Option<Vec<u8>>, exif: Option<Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = PngEncoder::new(&mut out);
    if let Some(icc) = icc {
        encoder.set_icc_profile(icc).unwrap();
    }
    if let Some(exif) = exif {
        encoder.set_exif_metadata(exif).unwrap();
    }
    encoder
        .write_image(rgba, w, h, ExtendedColorType::Rgba8)
        .unwrap();
    out
}

fn jpeg(rgba: &[u8], w: u32, h: u32, exif: Vec<u8>) -> Vec<u8> {
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    let mut out = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut out, 95);
    encoder.set_exif_metadata(exif).unwrap();
    encoder
        .write_image(&rgb, w, h, ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn image(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .flat_map(|(x, y)| f(x, y))
        .collect()
}

fn gradient(w: u32, h: u32) -> Vec<u8> {
    image(w, h, |x, y| {
        [
            (x * 255 / (w - 1)) as u8,
            (y * 255 / (h - 1)) as u8,
            ((x + y) * 7 % 256) as u8,
            255,
        ]
    })
}

/// Four distinct quadrants plus a fine pattern, so any rotation or flip is detected exactly.
fn quadrants(w: u32, h: u32) -> Vec<u8> {
    image(w, h, |x, y| {
        let q = [[255, 0, 0], [0, 200, 0], [0, 0, 255], [240, 220, 0]]
            [(usize::from(y >= h / 2) << 1) | usize::from(x >= w / 2)];
        [q[0] ^ (x as u8 * 3), q[1] ^ (y as u8 * 5), q[2], 255]
    })
}

/// Four flat 32×16 color blocks (JPEG-friendly).
fn blocks(w: u32, h: u32) -> Vec<u8> {
    image(w, h, |x, y| {
        let q = [[220, 40, 40], [40, 180, 60], [50, 70, 210], [235, 215, 30]]
            [(usize::from(y >= h / 2) << 1) | usize::from(x >= w / 2)];
        [q[0], q[1], q[2], 255]
    })
}

fn alpha_ramp() -> Vec<u8> {
    const COLORS: [[u8; 3]; 8] = [
        [0, 0, 0],
        [255, 255, 255],
        [230, 76, 60],
        [58, 117, 196],
        [252, 181, 20],
        [17, 133, 72],
        [128, 128, 128],
        [1, 254, 99],
    ];
    image(256, 8, |x, y| {
        let [r, g, b] = COLORS[y as usize];
        [r, g, b, x as u8]
    })
}

/// Minimal EXIF (TIFF, big-endian) with a single Orientation tag.
fn exif_orientation(value: u16) -> Vec<u8> {
    let mut exif = b"MM\0\x2a".to_vec();
    exif.extend_from_slice(&8u32.to_be_bytes());
    exif.extend_from_slice(&1u16.to_be_bytes());
    exif.extend_from_slice(&0x0112u16.to_be_bytes());
    exif.extend_from_slice(&3u16.to_be_bytes());
    exif.extend_from_slice(&1u32.to_be_bytes());
    exif.extend_from_slice(&value.to_be_bytes());
    exif.extend_from_slice(&[0, 0]);
    exif.extend_from_slice(&0u32.to_be_bytes());
    exif
}

/// An ICC v2 display profile (RGB → XYZ matrix + gamma 2.2 curves) like sRGB's, but with the red
/// and green colorants swapped, so a decoder that applies it changes the colors visibly.
fn swapped_profile() -> Vec<u8> {
    fn s15(v: f64) -> [u8; 4] {
        ((v * 65536.0).round() as i32).to_be_bytes()
    }
    fn xyz(x: f64, y: f64, z: f64) -> Vec<u8> {
        [b"XYZ ".as_slice(), &[0; 4], &s15(x), &s15(y), &s15(z)].concat()
    }
    let curve = [
        b"curv".as_slice(),
        &[0; 4],
        &1u32.to_be_bytes(),
        &0x0233u16.to_be_bytes(),
        &[0, 0],
    ]
    .concat();
    let text = |s: &str| [b"text".as_slice(), &[0; 4], s.as_bytes(), &[0]].concat();
    let desc = {
        let ascii = b"rekolor swapped R/G test profile\0";
        let mut d = [
            b"desc".as_slice(),
            &[0; 4],
            &(ascii.len() as u32).to_be_bytes(),
            ascii,
        ]
        .concat();
        d.extend_from_slice(&[0; 8]); // Unicode language code and count (none)
        d.extend_from_slice(&[0; 3]); // ScriptCode code and count (none)
        d.extend_from_slice(&[0; 67]); // ScriptCode string
        d
    };
    let tags: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"desc", desc),
        (b"wtpt", xyz(0.9642, 1.0, 0.8249)),
        // sRGB's D50 colorants, with red and green swapped.
        (b"rXYZ", xyz(0.3851, 0.7169, 0.0971)),
        (b"gXYZ", xyz(0.4361, 0.2225, 0.0139)),
        (b"bXYZ", xyz(0.1431, 0.0606, 0.7141)),
        (b"rTRC", curve.clone()),
        (b"gTRC", curve.clone()),
        (b"bTRC", curve),
        (b"cprt", text("No copyright, test data")),
    ];
    let table_len = 4 + 12 * tags.len();
    let mut offset = 128 + table_len;
    let mut table = (tags.len() as u32).to_be_bytes().to_vec();
    let mut data = Vec::new();
    for (sig, bytes) in &tags {
        table.extend_from_slice(*sig);
        table.extend_from_slice(&(offset as u32).to_be_bytes());
        table.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        data.extend_from_slice(bytes);
        let pad = (4 - bytes.len() % 4) % 4;
        data.extend(std::iter::repeat_n(0, pad));
        offset += bytes.len() + pad;
    }
    let total = 128 + table.len() + data.len();
    let mut header = Vec::with_capacity(128);
    header.extend_from_slice(&(total as u32).to_be_bytes());
    header.extend_from_slice(&[0; 4]); // preferred CMM
    header.extend_from_slice(&0x0210_0000u32.to_be_bytes()); // version 2.1
    header.extend_from_slice(b"mntrRGB XYZ ");
    header.extend_from_slice(&[0x07, 0xea, 0, 10, 0, 8, 0, 0, 0, 0, 0, 0]); // 2026-10-08
    header.extend_from_slice(b"acsp");
    header.extend_from_slice(&[0; 24]); // platform, flags, manufacturer, model, attributes
    header.extend_from_slice(&[0; 4]); // rendering intent: perceptual
    header.extend_from_slice(&[s15(0.9642), s15(1.0), s15(0.8249)].concat()); // PCS illuminant
    header.extend_from_slice(&[0; 4]); // creator
    header.extend_from_slice(&[0; 44]); // profile ID + reserved
    assert_eq!(header.len(), 128);
    [header, table, data].concat()
}
