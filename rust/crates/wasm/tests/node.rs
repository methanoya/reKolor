//! Runs inside WASM: `wasm-pack test --node crates/wasm` (X3 b).
//!
//! Proves the interface on the WASM target: the baseline (R9) reproduced inside WASM (native
//! and WASM float math agree), malformed input coming back as error values while the module keeps
//! working, and buffer-length checks.

#![cfg(target_arch = "wasm32")]

#[path = "../../../testdata/generator.rs"]
mod generator;

use rekolor_wasm::{
    ErrorKind, ImageStats, Mapping, Outcome, Palette, PaletteData, PaletteEntry, PaletteMatch,
    PaletteMatches, Pick, RecolorRequest, RecolorStats, Rgb, Rgba, SourceImage,
};
use serde::de::DeserializeOwned;
use tsify::{Ts, Tsify};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

/// The baseline outputs, compiled in (WASM tests have no file system).
macro_rules! baseline_table {
    ($($fixture:literal => [$($set:literal),*]);* $(;)?) => {
        &[$($((
            $fixture,
            $set,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../testdata/baseline/recolor/",
                $fixture, "__", $set, ".png"
            )) as &[u8],
        )),*),*]
    };
}

const BASELINE: &[(&str, &str, &[u8])] = baseline_table! {
    "alpha_ramp" => ["none", "one", "calendar3", "screenshot8", "swapped4"];
    "edges" => ["none", "one", "calendar3", "screenshot8", "swapped4"];
    "picks" => ["none", "one", "calendar3", "screenshot8", "swapped4"];
    "gradient" => ["none", "one", "calendar3", "screenshot8", "swapped4"];
    "noise" => ["none", "one", "calendar3", "screenshot8", "swapped4"];
    "transparent" => ["none", "one", "calendar3", "screenshot8", "swapped4"];
};

const IMAGE_INFO: &str = include_str!("../../../testdata/baseline/image-info.tsv");

fn decode_png(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("PNG header");
    let mut buf = vec![0; reader.output_buffer_size().expect("PNG buffer size")];
    let info = reader.next_frame(&mut buf).expect("PNG frame");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

/// Reads a returned outcome back into Rust.
fn read<T: Tsify + DeserializeOwned>(js: impl Into<JsValue>) -> Outcome<T> {
    Ts::<Outcome<T>>::new_unchecked(js.into())
        .to_rust()
        .expect("outcome deserializes")
}

fn ok<T: std::fmt::Debug>(outcome: Outcome<T>) -> T {
    match outcome {
        Outcome::Ok { value } => value,
        Outcome::Error { error } => panic!("expected ok, got {error:?}"),
    }
}

fn error_kind<T: std::fmt::Debug>(outcome: Outcome<T>) -> ErrorKind {
    match outcome {
        Outcome::Error { error } => error.kind,
        Outcome::Ok { value } => panic!("expected an error, got {value:?}"),
    }
}

fn rgb([r, g, b]: [u8; 3]) -> Rgb {
    Rgb { r, g, b }
}

fn request(set: &str) -> Ts<RecolorRequest> {
    let mappings = generator::mapping_set(set)
        .pairs
        .iter()
        .map(|&(source, ink)| Mapping {
            source: rgb(source),
            ink: rgb(ink),
        })
        .collect();
    Ts::from_rust(&RecolorRequest { mappings }).unwrap()
}

fn small_palette() -> Palette {
    let entry = |name: &str, c| PaletteEntry {
        name: name.into(),
        rgb: rgb(c),
    };
    Palette::from_data(PaletteData {
        entries: vec![
            entry("Pure White (non-palette)", [255, 255, 255]),
            entry("Pantone 179", [226, 61, 40]),
            entry("Pantone 285", [58, 117, 196]),
        ],
    })
    .unwrap()
}

/// Reads a property of a returned JS object.
fn property(object: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(object, &key.into()).unwrap()
}

#[wasm_bindgen_test]
fn recolor_reproduces_the_baseline_in_wasm() {
    assert_eq!(BASELINE.len(), 30);
    for &(fixture_name, set, png_bytes) in BASELINE {
        let fixture = generator::fixture(fixture_name);
        let image =
            SourceImage::from_rgba(fixture.rgba.clone(), fixture.width, fixture.height).unwrap();
        let mut out = vec![0; fixture.rgba.len()];
        let stats: RecolorStats = ok(read(image.recolor(request(set), &mut out)));
        assert_eq!(
            stats.exact + stats.nearest,
            f64::from(fixture.width * fixture.height)
        );

        let (w, h, expected) = decode_png(png_bytes);
        assert_eq!((w, h), (fixture.width, fixture.height));
        let differing = expected
            .as_chunks::<4>()
            .0
            .iter()
            .zip(out.as_chunks::<4>().0)
            .filter(|(e, a)| e != a)
            .count();
        assert_eq!(
            differing, 0,
            "{fixture_name} × {set}: {differing} pixels differ"
        );
    }
}

#[wasm_bindgen_test]
fn analyze_reproduces_image_info_in_wasm() {
    for line in IMAGE_INFO.lines().filter(|l| !l.starts_with('#')) {
        let cols: Vec<&str> = line.split('\t').collect();
        let fixture = generator::fixture(cols[0]);
        let image =
            SourceImage::from_rgba(fixture.rgba.clone(), fixture.width, fixture.height).unwrap();
        let stats: ImageStats = image.analyze().to_rust().unwrap();
        let expected: Vec<f64> = cols[1..].iter().map(|v| v.parse().unwrap()).collect();
        assert_eq!(
            vec![
                f64::from(stats.width),
                f64::from(stats.height),
                stats.rgb_colors,
                stats.rgba_colors
            ],
            expected,
            "{}",
            cols[0]
        );
    }
}

#[wasm_bindgen_test]
fn malformed_request_is_an_error_value_and_the_module_keeps_working() {
    let fixture = generator::fixture("edges");
    let image =
        SourceImage::from_rgba(fixture.rgba.clone(), fixture.width, fixture.height).unwrap();
    let mut out = vec![0; fixture.rgba.len()];

    for bad in [
        JsValue::from_str("not a request"),
        JsValue::from(42),
        JsValue::NULL,
        js_sys::JSON::parse(
            r#"{"mappings":[{"source":{"r":300,"g":0,"b":0},"ink":{"r":0,"g":0,"b":0}}]}"#,
        )
        .unwrap(),
    ] {
        let outcome: Outcome<RecolorStats> = read(image.recolor(Ts::new_unchecked(bad), &mut out));
        assert_eq!(error_kind(outcome), ErrorKind::InvalidInput);
    }

    // The same image object, and the module, still work.
    let stats: RecolorStats = ok(read(image.recolor(request("calendar3"), &mut out)));
    assert_eq!(stats.exact + stats.nearest, 96.0 * 96.0);
}

#[wasm_bindgen_test]
fn buffer_lengths_and_dimensions_are_checked() {
    assert_eq!(
        SourceImage::from_rgba(vec![0; 7], 1, 2).unwrap_err().kind,
        ErrorKind::BufferLength
    );
    assert_eq!(
        SourceImage::from_rgba(vec![], 0, 5).unwrap_err().kind,
        ErrorKind::EmptyImage
    );

    let image = SourceImage::from_rgba(vec![0; 8], 2, 1).unwrap();
    let mut short = [0u8; 4];
    let outcome: Outcome<RecolorStats> = read(image.recolor(request("none"), &mut short));
    assert_eq!(error_kind(outcome), ErrorKind::OutputLength);
}

#[wasm_bindgen_test]
fn create_factories_return_outcome_objects() {
    let ok_image = JsValue::from(SourceImage::create(vec![1, 2, 3, 4], 1.0, 1.0));
    assert_eq!(property(&ok_image, "status"), "ok");
    assert!(property(&ok_image, "value").is_object());

    let bad_image = JsValue::from(SourceImage::create(vec![1, 2, 3], 1.0, 1.0));
    assert_eq!(property(&bad_image, "status"), "error");
    assert_eq!(
        property(&property(&bad_image, "error"), "kind"),
        "bufferLength"
    );

    let bad_palette = JsValue::from(Palette::create(Ts::new_unchecked(JsValue::from(42))));
    assert_eq!(property(&bad_palette, "status"), "error");
    assert_eq!(
        property(&property(&bad_palette, "error"), "kind"),
        "invalidInput"
    );

    let empty = Ts::from_rust(&PaletteData { entries: vec![] }).unwrap();
    let empty_palette = JsValue::from(Palette::create(empty));
    assert_eq!(
        property(&property(&empty_palette, "error"), "kind"),
        "emptyPalette"
    );
}

#[wasm_bindgen_test]
fn pick_reads_the_source_pixel_and_warns_on_mismatch() {
    let palette = small_palette();
    assert_eq!(palette.length(), 3);
    // Two pixels: transparent, then opaque red.
    let image = SourceImage::from_rgba(vec![0, 0, 0, 0, 230, 76, 60, 255], 2, 1).unwrap();

    let p: Pick = ok(read(image.pick(1.0, 0.0, None, &palette)));
    assert_eq!(
        p.pixel,
        Rgba {
            r: 230,
            g: 76,
            b: 60,
            a: 255
        }
    );
    assert_eq!(p.matching, rgb([230, 76, 60]));
    assert_eq!(p.suggestion.name, "Pantone 179");
    assert_eq!(p.suggestion.index, 1);
    assert_eq!(p.mismatch, None);

    // Transparent is matched as white.
    let p: Pick = ok(read(image.pick(0.0, 0.0, None, &palette)));
    assert_eq!(p.matching, rgb([255, 255, 255]));
    assert!(p.suggestion.non_palette);

    // The caller saw something else: still a valid pick, plus a warning.
    let seen = Ts::from_rust(&Rgba {
        r: 230,
        g: 76,
        b: 160,
        a: 255,
    })
    .unwrap();
    let p: Pick = ok(read(image.pick(1.0, 0.0, Some(seen), &palette)));
    let mismatch = p.mismatch.expect("mismatch warning");
    assert_eq!(mismatch.max_channel_difference, 100);
    assert_eq!(
        p.pixel,
        Rgba {
            r: 230,
            g: 76,
            b: 60,
            a: 255
        }
    );

    // Within the tolerance: no warning.
    let close = Ts::from_rust(&Rgba {
        r: 232,
        g: 76,
        b: 60,
        a: 255,
    })
    .unwrap();
    let p: Pick = ok(read(image.pick(1.0, 0.0, Some(close), &palette)));
    assert_eq!(p.mismatch, None);

    let outcome: Outcome<Pick> = read(image.pick(2.0, 0.0, None, &palette));
    assert_eq!(error_kind(outcome), ErrorKind::OutOfBounds);

    let bad_seen = Ts::new_unchecked(JsValue::from_str("red"));
    let outcome: Outcome<Pick> = read(image.pick(1.0, 0.0, Some(bad_seen), &palette));
    assert_eq!(error_kind(outcome), ErrorKind::InvalidInput);
}

#[wasm_bindgen_test]
fn palette_suggest_and_nearest() {
    let palette = small_palette();
    let red = Ts::from_rust(&rgb([230, 76, 60])).unwrap();

    let m: PaletteMatch = ok(read(palette.suggest(red.clone())));
    assert_eq!((m.index, m.name.as_str()), (1, "Pantone 179"));

    let all: PaletteMatches = ok(read(palette.nearest(red, 2.0)));
    assert_eq!(all.matches.len(), 2);
    assert_eq!(all.matches[0].index, 1);
    assert!(all.matches[0].delta_e <= all.matches[1].delta_e);

    let outcome: Outcome<PaletteMatch> =
        read(palette.suggest(Ts::new_unchecked(JsValue::UNDEFINED)));
    assert_eq!(error_kind(outcome), ErrorKind::InvalidInput);
}

#[wasm_bindgen_test]
fn numbers_must_be_whole_finite_and_in_u32_range() {
    // Review fix F1: these would otherwise be truncated or wrapped into valid-looking values.
    let one = || vec![5, 6, 7, 255];
    for (w, h) in [
        (1.5, 1.0),
        (4294967297.0, 1.0),
        (f64::NAN, 1.0),
        (f64::INFINITY, 1.0),
        (-1.0, 1.0),
        (1.0, 0.5),
    ] {
        let created = JsValue::from(SourceImage::create(one(), w, h));
        assert_eq!(property(&created, "status"), "error", "create({w}, {h})");
        assert_eq!(
            property(&property(&created, "error"), "kind"),
            "invalidInput"
        );
    }

    let image = SourceImage::from_rgba(one(), 1, 1).unwrap();
    let palette = small_palette();
    for (x, y) in [
        (f64::NAN, 0.0),
        (f64::INFINITY, 0.0),
        (-0.9, 0.0),
        (4294967296.0, 0.0),
        (0.5, 0.0),
        (0.0, -1.0),
    ] {
        let outcome: Outcome<Pick> = read(image.pick(x, y, None, &palette));
        assert_eq!(
            error_kind(outcome),
            ErrorKind::InvalidInput,
            "pick({x}, {y})"
        );
    }
    let black = || Ts::from_rust(&rgb([0, 0, 0])).unwrap();
    for k in [4294967296.0, 1.5, f64::NAN, -1.0] {
        let outcome: Outcome<PaletteMatches> = read(palette.nearest(black(), k));
        assert_eq!(
            error_kind(outcome),
            ErrorKind::InvalidInput,
            "nearest(k={k})"
        );
    }

    // Valid calls on the same objects still work; -0 counts as 0.
    let p: Pick = ok(read(image.pick(-0.0, 0.0, None, &palette)));
    assert_eq!(
        p.pixel,
        Rgba {
            r: 5,
            g: 6,
            b: 7,
            a: 255
        }
    );
    let m: PaletteMatches = ok(read(palette.nearest(black(), 1.0)));
    assert_eq!(m.matches.len(), 1);
}
