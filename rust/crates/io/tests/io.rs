//! `rekolor-io`: decoding, encoding and the M1 warnings.

#[path = "../../../testdata/generator.rs"]
mod generator;

use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};
use rekolor_io::{DecodeWarning, Error, decode, decode_file, encode_png};

/// A PNG with the given pixels and optional ICC profile / EXIF data, written by `image`.
fn png_with(
    rgba: &[u8],
    width: u32,
    height: u32,
    icc: Option<Vec<u8>>,
    exif: Option<Vec<u8>>,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = PngEncoder::new(&mut out);
    if let Some(icc) = icc {
        encoder.set_icc_profile(icc).unwrap();
    }
    if let Some(exif) = exif {
        encoder.set_exif_metadata(exif).unwrap();
    }
    encoder
        .write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .unwrap();
    out
}

/// Minimal EXIF (TIFF, big-endian) with a single Orientation tag.
fn exif_orientation(value: u16) -> Vec<u8> {
    let mut exif = b"MM\0\x2a".to_vec(); // big-endian TIFF header
    exif.extend_from_slice(&8u32.to_be_bytes()); // offset of the first IFD
    exif.extend_from_slice(&1u16.to_be_bytes()); // one entry
    exif.extend_from_slice(&0x0112u16.to_be_bytes()); // tag: Orientation
    exif.extend_from_slice(&3u16.to_be_bytes()); // type: SHORT
    exif.extend_from_slice(&1u32.to_be_bytes()); // count
    exif.extend_from_slice(&value.to_be_bytes()); // value
    exif.extend_from_slice(&[0, 0]); // padding to 4 bytes
    exif.extend_from_slice(&0u32.to_be_bytes()); // no next IFD
    exif
}

const A: [u8; 4] = [255, 0, 0, 255];
const B: [u8; 4] = [0, 0, 255, 255];

#[test]
fn png_round_trip_is_lossless_for_every_fixture() {
    for fixture in generator::fixtures() {
        let png = encode_png(&fixture.rgba, fixture.width, fixture.height).unwrap();
        let decoded = decode(&png).unwrap();
        assert_eq!(
            (decoded.width, decoded.height),
            (fixture.width, fixture.height)
        );
        assert_eq!(decoded.rgba, fixture.rgba, "{}", fixture.name);

        // The only possible warning for a plain PNG: semi-transparent pixels, counted exactly.
        let expected = fixture
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] != 0 && p[3] != 255)
            .count() as u64;
        let expected_warnings = if expected > 0 {
            vec![DecodeWarning::SemiTransparentPixels { count: expected }]
        } else {
            vec![]
        };
        assert_eq!(decoded.warnings, expected_warnings, "{}", fixture.name);
    }
}

#[test]
fn encode_checks_the_buffer_length() {
    assert!(matches!(
        encode_png(&[0; 7], 1, 2),
        Err(Error::InvalidImage(
            rekolor_core::Error::BufferLength { .. }
        ))
    ));
}

#[test]
fn sixteen_bit_and_grayscale_are_converted_to_rgba8() {
    let gray16 =
        image::ImageBuffer::<image::Luma<u16>, _>::from_raw(3, 1, vec![0, 32896, 65535]).unwrap();
    let mut png = Vec::new();
    gray16
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let decoded = decode(&png).unwrap();
    assert_eq!(
        decoded.rgba,
        [0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255]
    );

    let gray_alpha =
        image::ImageBuffer::<image::LumaA<u8>, _>::from_raw(1, 1, vec![200, 100]).unwrap();
    let mut png = Vec::new();
    gray_alpha
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let decoded = decode(&png).unwrap();
    assert_eq!(decoded.rgba, [200, 200, 200, 100]);
    assert_eq!(
        decoded.warnings,
        [DecodeWarning::SemiTransparentPixels { count: 1 }]
    );
}

#[test]
fn icc_profile_is_reported() {
    let profile = vec![7u8; 128]; // contents aren't interpreted
    let png = png_with(&A, 1, 1, Some(profile), None);
    let decoded = decode(&png).unwrap();
    assert_eq!(decoded.rgba, A);
    assert_eq!(decoded.warnings, [DecodeWarning::IccProfile { bytes: 128 }]);
}

#[test]
fn exif_orientation_is_applied_and_reported_png() {
    // 2×1 image [A B] with EXIF orientation 6 (rotate 90° clockwise) → 1×2, A on top.
    let rgba = [A, B].concat();
    let png = png_with(&rgba, 2, 1, None, Some(exif_orientation(6)));
    let decoded = decode(&png).unwrap();
    assert_eq!((decoded.width, decoded.height), (1, 2));
    assert_eq!(decoded.rgba, [A, B].concat());
    assert_eq!(
        decoded.warnings,
        [DecodeWarning::OrientationApplied { exif_value: 6 }]
    );

    // Orientation 3 (rotate 180°) keeps the size and reverses the row.
    let png = png_with(&rgba, 2, 1, None, Some(exif_orientation(3)));
    let decoded = decode(&png).unwrap();
    assert_eq!((decoded.width, decoded.height), (2, 1));
    assert_eq!(decoded.rgba, [B, A].concat());
}

#[test]
fn exif_orientation_is_applied_and_reported_jpeg() {
    // JPEG is lossy, so only the size and the warning are checked.
    let rgb = vec![200u8; 16 * 8 * 3];
    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 90);
    encoder.set_exif_metadata(exif_orientation(8)).unwrap();
    encoder
        .write_image(&rgb, 16, 8, ExtendedColorType::Rgb8)
        .unwrap();
    let decoded = decode(&jpeg).unwrap();
    assert_eq!((decoded.width, decoded.height), (8, 16));
    assert_eq!(
        decoded.warnings,
        [DecodeWarning::OrientationApplied { exif_value: 8 }]
    );
}

#[test]
fn decode_errors_are_typed() {
    assert!(matches!(decode(b"not an image"), Err(Error::UnknownFormat)));

    let png = encode_png(&generator::fixture("noise").rgba, 128, 96).unwrap();
    let truncated = &png[..png.len() / 2];
    assert!(matches!(decode(truncated), Err(Error::Decode(_))));

    let missing = Path::new("/nonexistent/rekolor/missing.png");
    match decode_file(missing) {
        Err(Error::Io { path, .. }) => assert_eq!(path, missing),
        other => panic!("expected an I/O error, got {other:?}"),
    }
}

#[test]
fn every_sample_image_decodes() {
    let samples = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples"));
    let mut files = Vec::new();
    collect_images(&samples, &mut files);
    files.sort();
    assert!(files.len() >= 19, "found {} sample images", files.len());
    for path in &files {
        let decoded = decode_file(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(decoded.width > 0 && decoded.height > 0);
        let warnings: Vec<String> = decoded.warnings.iter().map(|w| w.to_string()).collect();
        eprintln!(
            "{} {}×{} {:?}",
            path.strip_prefix(&samples).unwrap().display(),
            decoded.width,
            decoded.height,
            warnings
        );
    }
}

fn collect_images(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_images(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
            && !path.to_string_lossy().contains("-out-")
        {
            out.push(path);
        }
    }
}
