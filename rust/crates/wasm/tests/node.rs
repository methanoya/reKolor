//! Runs inside WASM: `wasm-pack test --node crates/wasm`.
//!
//! Proves the interface on the WASM target: the baseline reproduced inside WASM (native
//! and WASM float math agree), malformed input coming back as error values while the module keeps
//! working, and buffer-length checks. Also the ΔE fingerprint, the Sharma reference data and the
//! Pantone suggestion snapshot, all checked against the same files as the native tests.

// This whole file compiles only for the WASM target (`#![cfg(...)]` on the file), so a native
// `cargo test --workspace` skips it. `wasm-pack test --node` compiles the tests to WebAssembly and
// runs them in Node.js, which executes WASM the same way a browser does.
#![cfg(target_arch = "wasm32")]

#[path = "../../../testdata/generator.rs"]
mod generator;

use rekolor_core::Rgb8;
use rekolor_wasm::{
    ColorCount, ConfigExport, ConfigPick, ConfigSection, ConfigText, ConfigUnprinted, ErrorKind,
    ImageStats, Mapping, MaterialRange, Outcome, Palette, PaletteData, PaletteEntry, PaletteMatch,
    PaletteMatches, ParsedConfig, Pick, RecolorRequest, RecolorStats, ResolvedPicks, Rgb, Rgba,
    SourceImage, UnprintedCheck, UnprintedColors, composite, parse_config, serialize_config,
    unprinted_colors,
};
use serde::de::DeserializeOwned;
use tsify::{Ts, Tsify};
use wasm_bindgen::JsValue;
// `#[wasm_bindgen_test]` replaces `#[test]` for tests that run inside WASM.
use wasm_bindgen_test::wasm_bindgen_test;

// `macro_rules!` defines a macro: code that writes code at compile time. This one expands the
// table below into one `(fixture, set, PNG bytes)` entry per combination, embedding each baseline
// PNG with `include_bytes!`. `$(...),*` repeats a pattern for each comma-separated item.
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

// The exported functions return JavaScript values (`JsValue`), as the browser would receive
// them; the tests convert them back to Rust to inspect them. `new_unchecked` wraps the value
// without checking its type; `to_rust` then fails if the shape is wrong.
/// Reads a returned outcome back into Rust.
fn read<T: Tsify + DeserializeOwned>(js: impl Into<JsValue>) -> Outcome<T> {
    Ts::<Outcome<T>>::new_unchecked(js.into())
        .to_rust()
        .expect("outcome deserializes")
}

// The value of a successful outcome, or the error's kind; anything else fails the test.
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

const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};

/// A material argument.
fn material(c: Rgb) -> Ts<Rgb> {
    Ts::from_rust(&c).unwrap()
}

fn white() -> Ts<Rgb> {
    material(WHITE)
}

fn request(set: &str) -> Ts<RecolorRequest> {
    let mappings = generator::mapping_set(set)
        .pairs
        .iter()
        .map(|&(source, ink)| Mapping {
            source: rgb(source),
            ink: rgb(ink),
            delta_e: 0.0,
        })
        .collect();
    Ts::from_rust(&RecolorRequest {
        material_ranges: vec![],
        material: WHITE,
        mappings,
    })
    .unwrap()
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

// `object[key]` in JavaScript terms.
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
        let stats: ImageStats = ok(read(image.analyze(white())));
        let expected: Vec<f64> = cols[1..].iter().map(|v| v.parse().unwrap()).collect();
        assert_eq!(
            vec![
                f64::from(stats.width),
                f64::from(stats.height),
                stats.colors,
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

    let p: Pick = ok(read(image.pick(1.0, 0.0, None, white(), &palette)));
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
    let p: Pick = ok(read(image.pick(0.0, 0.0, None, white(), &palette)));
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
    let p: Pick = ok(read(image.pick(1.0, 0.0, Some(seen), white(), &palette)));
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
    let p: Pick = ok(read(image.pick(1.0, 0.0, Some(close), white(), &palette)));
    assert_eq!(p.mismatch, None);

    let outcome: Outcome<Pick> = read(image.pick(2.0, 0.0, None, white(), &palette));
    assert_eq!(error_kind(outcome), ErrorKind::OutOfBounds);

    let bad_seen = Ts::new_unchecked(JsValue::from_str("red"));
    let outcome: Outcome<Pick> = read(image.pick(1.0, 0.0, Some(bad_seen), white(), &palette));
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
    // These would otherwise be truncated or wrapped into valid-looking values.
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
        let outcome: Outcome<Pick> = read(image.pick(x, y, None, white(), &palette));
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
    let p: Pick = ok(read(image.pick(-0.0, 0.0, None, white(), &palette)));
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

#[wasm_bindgen_test]
fn zero_and_one_mapping_follow_the_contract() {
    // No mappings → the composited copy; one mapping → every pixel takes that ink.
    let image = SourceImage::from_rgba(vec![230, 76, 60, 255, 9, 9, 9, 0], 2, 1).unwrap();
    let mut out = [0u8; 8];

    let none = Ts::from_rust(&RecolorRequest {
        material_ranges: vec![],
        material: WHITE,
        mappings: vec![],
    })
    .unwrap();
    let stats: RecolorStats = ok(read(image.recolor(none, &mut out)));
    assert_eq!(out, [230, 76, 60, 255, 255, 255, 255, 255]);
    assert_eq!((stats.exact, stats.nearest), (0.0, 2.0));

    let one = Ts::from_rust(&RecolorRequest {
        material_ranges: vec![],
        material: WHITE,
        mappings: vec![Mapping {
            source: rgb([230, 76, 60]),
            ink: rgb([40, 120, 200]),
            delta_e: 0.0,
        }],
    })
    .unwrap();
    let stats: RecolorStats = ok(read(image.recolor(one, &mut out)));
    assert_eq!(out, [40, 120, 200, 255, 40, 120, 200, 255]);
    assert_eq!((stats.exact, stats.nearest), (1.0, 1.0));
}

#[wasm_bindgen_test]
fn a_mappings_delta_e_captures_nearby_colors_and_is_checked() {
    // Red, and a red close to it (ΔE under 5); the second mapping's ink is almost that near red.
    let image = SourceImage::from_rgba(vec![230, 76, 60, 255, 226, 82, 66, 255], 2, 1).unwrap();
    let request = |delta_e| {
        Ts::from_rust(&RecolorRequest {
            material_ranges: vec![],
            material: WHITE,
            mappings: vec![
                Mapping {
                    source: rgb([230, 76, 60]),
                    ink: rgb([0, 0, 0]),
                    delta_e,
                },
                Mapping {
                    source: rgb([40, 120, 200]),
                    ink: rgb([227, 81, 65]),
                    delta_e: 0.0,
                },
            ],
        })
        .unwrap()
    };
    let mut out = [0u8; 8];
    // No radius: the near red takes the nearest ink.
    let stats: RecolorStats = ok(read(image.recolor(request(0.0), &mut out)));
    assert_eq!(out, [0, 0, 0, 255, 227, 81, 65, 255]);
    assert_eq!(
        (stats.exact, stats.captured, stats.nearest),
        (1.0, 0.0, 1.0)
    );
    // A radius of 5 around red captures it.
    let stats: RecolorStats = ok(read(image.recolor(request(5.0), &mut out)));
    assert_eq!(out, [0, 0, 0, 255, 0, 0, 0, 255]);
    assert_eq!(
        (stats.exact, stats.captured, stats.nearest),
        (1.0, 1.0, 0.0)
    );

    for bad in [-1.0, 100.5, f32::NAN] {
        assert_eq!(
            error_kind::<RecolorStats>(read(image.recolor(request(bad), &mut out))),
            ErrorKind::InvalidInput,
            "deltaE {bad}"
        );
    }
}

#[wasm_bindgen_test]
fn configs_carry_each_picks_delta_e() {
    let text = "[[palette]]\nsize = 2\npicks = [\n\
                { rgba = [230, 76, 60, 255], ink = \"Pantone 179\", delta_e = 12.5 },\n\
                { rgba = [40, 120, 200, 255], ink = \"Pantone 285\" },\n]\n";
    let parsed: ParsedConfig = ok(read(parse_config(text)));
    let radii = |picks: &[ConfigPick]| picks.iter().map(|p| p.delta_e).collect::<Vec<_>>();
    assert_eq!(radii(&parsed.sections[0].picks), [12.5, 0.0]);
    let resolved: ResolvedPicks = ok(read(
        small_palette().resolve_section(Ts::from_rust(&parsed.sections[0]).unwrap(), white()),
    ));
    assert_eq!(
        resolved.picks.iter().map(|p| p.delta_e).collect::<Vec<_>>(),
        [12.5, 0.0]
    );
    // Exported back: written only where set.
    let export = ConfigExport {
        unprinted: vec![],
        image_name: "x.png".into(),
        material: WHITE,
        picks: parsed.sections[0].picks.clone(),
    };
    let out: ConfigText = ok(read(serialize_config(Ts::from_rust(&export).unwrap())));
    assert!(
        out.text.contains("ink = \"Pantone 179\", delta_e = 12.5 }"),
        "{}",
        out.text
    );
    assert!(out.text.contains("ink = \"Pantone 285\" }"), "{}", out.text);
}

#[wasm_bindgen_test]
fn delta_e_matches_the_native_fingerprint() {
    // Bit-identical ΔE on native and WASM. The file was recorded natively.
    let recorded = include_str!("../../../testdata/baseline/delta-e-fingerprint.tsv");
    let now = generator::delta_e_fingerprint(|x, y| rekolor_core::delta_e_2000(x.into(), y.into()));
    assert_eq!(now, recorded, "WASM ΔE differs from the native fingerprint");
}

#[wasm_bindgen_test]
fn delta_e_matches_the_sharma_reference_data() {
    let data = include_str!("../../../testdata/ciede2000-sharma.tsv");
    let mut pairs = 0;
    for line in data.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<f32> = line.split('\t').map(|x| x.parse().unwrap()).collect();
        let x = rekolor_core::Lab::new(v[0], v[1], v[2]);
        let y = rekolor_core::Lab::new(v[3], v[4], v[5]);
        let actual = rekolor_core::delta_e_2000_lab(x, y);
        assert!((actual - v[6]).abs() <= 1e-4, "{line}: got {actual}");
        pairs += 1;
    }
    assert_eq!(pairs, 34);
}

#[wasm_bindgen_test]
fn suggestions_match_the_snapshot() {
    // Same check as the native `suggestions` test: names and ΔE to 6 decimals.
    let json: serde_json::Value =
        serde_json::from_str(include_str!("../../../../palettes/pantone.json")).unwrap();
    let entries = json
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| {
            let c: Vec<u8> = value["rgb"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect();
            rekolor_core::PaletteEntry::new(name.clone(), rekolor_core::Rgb8::new(c[0], c[1], c[2]))
        })
        .collect();
    let palette = rekolor_core::Palette::new(entries).unwrap();
    let snapshot = include_str!("../../../testdata/baseline/pantone-suggestions.tsv");
    let mut rows = 0;
    for line in snapshot.lines().filter(|l| !l.starts_with('#')) {
        let cols: Vec<&str> = line.split('\t').collect();
        let c: Vec<u8> = cols[..3].iter().map(|v| v.parse().unwrap()).collect();
        let m = palette.suggest(rekolor_core::Rgb8::new(c[0], c[1], c[2]));
        let actual = format!("{}\t{:.6}", m.entry.name, m.delta_e);
        assert_eq!(actual, format!("{}\t{}", cols[3], cols[4]), "{line}");
        rows += 1;
    }
    assert_eq!(rows, 5096);
}

#[wasm_bindgen_test]
fn nearest_ink_ties_go_to_the_earlier_mapping() {
    // Ties in WASM: the exact tie from the core test (same bits on both targets).
    let gray = SourceImage::from_rgba(vec![128, 128, 128, 255], 1, 1).unwrap();
    let (a, b) = (rgb([0, 2, 227]), rgb([0, 4, 0]));
    assert_eq!(
        rekolor_core::delta_e_2000(Rgb8::new(128, 128, 128), Rgb8::new(0, 2, 227)).to_bits(),
        rekolor_core::delta_e_2000(Rgb8::new(128, 128, 128), Rgb8::new(0, 4, 0)).to_bits(),
        "precondition: an exact tie"
    );
    let mut out = [0u8; 4];
    for (first, second, expected) in [(a, b, [0, 2, 227, 255]), (b, a, [0, 4, 0, 255])] {
        let request = Ts::from_rust(&RecolorRequest {
            material_ranges: vec![],
            material: WHITE,
            mappings: vec![
                Mapping {
                    source: rgb([230, 76, 60]),
                    ink: first,
                    delta_e: 0.0,
                },
                Mapping {
                    source: rgb([40, 120, 200]),
                    ink: second,
                    delta_e: 0.0,
                },
            ],
        })
        .unwrap();
        let _: RecolorStats = ok(read(gray.recolor(request, &mut out)));
        assert_eq!(out, expected);
    }
}

/// The Pantone palette as the app builds it (file order kept).
fn pantone_palette() -> Palette {
    let json: serde_json::Value =
        serde_json::from_str(include_str!("../../../../palettes/pantone.json")).unwrap();
    let entries = json
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| {
            let c: Vec<u8> = value["rgb"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect();
            PaletteEntry {
                name: name.clone(),
                rgb: rgb([c[0], c[1], c[2]]),
            }
        })
        .collect();
    Palette::from_data(PaletteData { entries }).unwrap()
}

const SAMPLE_CONFIGS: &[&str] = &[
    include_str!("../../../../samples/selected/04-tiger.palettes.toml"),
    include_str!("../../../../samples/arbitrary/07-alpha-hue.palettes.toml"),
    include_str!("../../../../samples/others/icon-calendar.palettes.toml"),
];

#[wasm_bindgen_test]
fn sample_configs_parse_resolve_and_round_trip() {
    // The app reads the golden-set configs exactly like the CLI.
    let palette = pantone_palette();
    for text in SAMPLE_CONFIGS {
        let parsed: ParsedConfig = ok(read(parse_config(text)));
        assert_eq!(
            parsed.sections.iter().map(|s| s.size).collect::<Vec<_>>(),
            [3, 7, 16]
        );
        for section in &parsed.sections {
            let resolved: ResolvedPicks = ok(read(
                palette.resolve_section(Ts::from_rust(section).unwrap(), material(parsed.material)),
            ));
            assert_eq!(resolved.picks.len(), section.picks.len());
            for (pick, r) in section.picks.iter().zip(&resolved.picks) {
                assert_eq!(r.pixel, pick.rgba);
                assert_eq!(r.ink.name, pick.ink);
            }
            // Export (one section, size = number of picks) and read it back.
            let export = ConfigExport {
                unprinted: vec![],
                image_name: "x.png".into(),
                material: parsed.material,
                picks: section.picks.clone(),
            };
            let text: ConfigText = ok(read(serialize_config(Ts::from_rust(&export).unwrap())));
            assert!(
                text.text
                    .starts_with("# Palette exported from the reKolor web app for x.png.")
            );
            let again: ParsedConfig = ok(read(parse_config(&text.text)));
            assert_eq!(again.material, parsed.material);
            assert_eq!(
                again.sections,
                [ConfigSection {
                    size: section.picks.len() as u32,
                    picks: section.picks.clone()
                }]
            );
        }
    }
}

#[wasm_bindgen_test]
fn resolved_picks_are_composited_in_rust() {
    let palette = pantone_palette();
    let section = ConfigSection {
        size: 2,
        picks: vec![
            ConfigPick {
                rgba: Rgba {
                    r: 9,
                    g: 9,
                    b: 9,
                    a: 0,
                },
                ink: "Pure White (non-palette)".into(),
                delta_e: 0.0,
            },
            ConfigPick {
                rgba: Rgba {
                    r: 203,
                    g: 0,
                    b: 0,
                    a: 100,
                },
                ink: "Pantone 185".into(),
                delta_e: 0.0,
            },
        ],
    };
    let resolved: ResolvedPicks = ok(read(
        palette.resolve_section(Ts::from_rust(&section).unwrap(), white()),
    ));
    assert_eq!(resolved.picks[0].matching, rgb([255, 255, 255]));
    assert_eq!(resolved.picks[0].ink.delta_e, 0.0);
    // The truncating core formula: (255 − 100) + 100 × 203 / 255 = 234 (not 235).
    assert_eq!(resolved.picks[1].matching, rgb([234, 155, 155]));
    assert_eq!(resolved.picks[1].ink.name, "Pantone 185");
}

#[wasm_bindgen_test]
fn bad_configs_are_invalid_config_errors() {
    let palette = pantone_palette();
    for text in [
        "",
        "[[palette]\n",
        "[[palette]]\nsize = 3\npicks = []\nextra = 1\n",
        "[[palette]]\nsize = 3\npicks = []\n[[palette]]\nsize = 3\npicks = []\n",
        "[[palette]]\nsize = 1\npicks = [{ rgba = [0, 0, 0, 255], ink = \"A\" }, { rgba = [1, 0, 0, 255], ink = \"B\" }]\n",
    ] {
        assert_eq!(
            error_kind::<ParsedConfig>(read(parse_config(text))),
            ErrorKind::InvalidConfig,
            "{text:?}"
        );
    }
    let oversized = format!("# {}\n", "x".repeat(256 * 1024));
    assert_eq!(
        error_kind::<ParsedConfig>(read(parse_config(&oversized))),
        ErrorKind::InvalidConfig
    );
    let unknown = ConfigSection {
        size: 1,
        picks: vec![ConfigPick {
            rgba: Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
            ink: "Pantone 99999".into(),
            delta_e: 0.0,
        }],
    };
    assert_eq!(
        error_kind::<ResolvedPicks>(read(
            palette.resolve_section(Ts::from_rust(&unknown).unwrap(), white())
        )),
        ErrorKind::InvalidConfig
    );
}

#[wasm_bindgen_test]
fn color_count_matches_analyze_and_mappings_are_capped() {
    let fixture = generator::fixture("noise");
    let image =
        SourceImage::from_rgba(fixture.rgba.clone(), fixture.width, fixture.height).unwrap();
    let stats: ImageStats = ok(read(image.analyze(white())));
    let count: ColorCount = ok(read(image.color_count(white())));
    assert_eq!(count.colors, stats.colors);

    let mut out = vec![0u8; fixture.rgba.len()];
    let mapping = |i: u32| Mapping {
        source: rgb([i as u8, (i >> 8) as u8, 1]),
        ink: rgb([0, 0, 0]),
        delta_e: 0.0,
    };
    let allowed = Ts::from_rust(&RecolorRequest {
        material_ranges: vec![],
        material: WHITE,
        mappings: (0..256).map(mapping).collect(),
    })
    .unwrap();
    let _: RecolorStats = ok(read(image.recolor(allowed, &mut out)));
    let too_many = Ts::from_rust(&RecolorRequest {
        material_ranges: vec![],
        material: WHITE,
        mappings: (0..257).map(mapping).collect(),
    })
    .unwrap();
    assert_eq!(
        error_kind::<RecolorStats>(read(image.recolor(too_many, &mut out))),
        ErrorKind::TooManyMappings
    );
}

#[wasm_bindgen_test]
fn every_call_composites_over_the_given_material() {
    let black = rgb([0, 0, 0]);
    let palette = pantone_palette();
    // Transparent (hiding 9,9,9), opaque red, translucent red.
    let image =
        SourceImage::from_rgba(vec![9, 9, 9, 0, 230, 76, 60, 255, 203, 0, 0, 100], 3, 1).unwrap();

    let p: Pick = ok(read(image.pick(0.0, 0.0, None, material(black), &palette)));
    assert_eq!(p.matching, black);
    assert_eq!(p.suggestion.name, "Pure Black (non-palette)");
    let p: Pick = ok(read(image.pick(2.0, 0.0, None, material(black), &palette)));
    assert_eq!(p.matching, rgb([79, 0, 0]));

    // No mappings: the copy composited over the material.
    let mut out = [0u8; 12];
    let none = Ts::from_rust(&RecolorRequest {
        material_ranges: vec![],
        mappings: vec![],
        material: black,
    })
    .unwrap();
    let _: RecolorStats = ok(read(image.recolor(none, &mut out)));
    assert_eq!(out, [0, 0, 0, 255, 230, 76, 60, 255, 79, 0, 0, 255]);

    let on_black: ColorCount = ok(read(image.color_count(material(black))));
    let on_white: ColorCount = ok(read(image.color_count(white())));
    assert_eq!((on_black.colors, on_white.colors), (3.0, 3.0));
    let stats: ImageStats = ok(read(image.analyze(material(black))));
    assert_eq!(stats.colors, 3.0);

    let pixel = |r, g, b, a| Ts::from_rust(&Rgba { r, g, b, a }).unwrap();
    let c: Rgb = ok(read(composite(pixel(203, 0, 0, 100), material(black))));
    assert_eq!(c, rgb([79, 0, 0]));
    let c: Rgb = ok(read(composite(pixel(203, 0, 0, 100), white())));
    assert_eq!(c, rgb([234, 155, 155]));

    // Required and checked: a request without a material, or a malformed one, is an error value.
    let missing = js_sys::JSON::parse(r#"{"mappings":[]}"#).unwrap();
    assert_eq!(
        error_kind::<RecolorStats>(read(image.recolor(Ts::new_unchecked(missing), &mut out))),
        ErrorKind::InvalidInput
    );
    let bad = || Ts::new_unchecked(JsValue::from_str("white"));
    assert_eq!(
        error_kind::<Pick>(read(image.pick(0.0, 0.0, None, bad(), &palette))),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        error_kind::<ColorCount>(read(image.color_count(bad()))),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        error_kind::<Rgb>(read(composite(pixel(1, 2, 3, 4), bad()))),
        ErrorKind::InvalidInput
    );
}

#[wasm_bindgen_test]
fn configs_carry_the_material() {
    let palette = pantone_palette();
    // Absent: white.
    let parsed: ParsedConfig = ok(read(parse_config("[[palette]]\nsize = 1\npicks = []\n")));
    assert_eq!(parsed.material, WHITE);

    let text = "material = [0, 0, 0]\n\n[[palette]]\nsize = 1\npicks = [\n\
                { rgba = [9, 9, 9, 0], ink = \"Pure Black (non-palette)\" },\n]\n";
    let parsed: ParsedConfig = ok(read(parse_config(text)));
    assert_eq!(parsed.material, rgb([0, 0, 0]));
    let resolved: ResolvedPicks = ok(read(palette.resolve_section(
        Ts::from_rust(&parsed.sections[0]).unwrap(),
        material(parsed.material),
    )));
    assert_eq!(resolved.picks[0].matching, rgb([0, 0, 0]));
    assert_eq!(resolved.picks[0].ink.delta_e, 0.0);

    // Always written, white too.
    for c in [rgb([0, 0, 0]), WHITE] {
        let export = ConfigExport {
            unprinted: vec![],
            image_name: "x.png".into(),
            material: c,
            picks: parsed.sections[0].picks.clone(),
        };
        let out: ConfigText = ok(read(serialize_config(Ts::from_rust(&export).unwrap())));
        let line = format!("\nmaterial = [{}, {}, {}]\n", c.r, c.g, c.b);
        assert!(out.text.contains(&line), "{}", out.text);
    }

    assert_eq!(
        error_kind::<ParsedConfig>(read(parse_config("material = [1, 2]\n"))),
        ErrorKind::InvalidConfig
    );
}

#[wasm_bindgen_test]
fn material_ranges_leave_pixels_transparent_and_are_checked() {
    // Transparent (hiding 9,9,9), opaque red, opaque black; on black with the material's own range.
    let image =
        SourceImage::from_rgba(vec![9, 9, 9, 0, 230, 76, 60, 255, 0, 0, 0, 255], 3, 1).unwrap();
    let request = |ranges: Vec<MaterialRange>| {
        Ts::from_rust(&RecolorRequest {
            mappings: vec![],
            material: rgb([0, 0, 0]),
            material_ranges: ranges,
        })
        .unwrap()
    };
    let black = |delta_e| MaterialRange {
        pixel: Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        delta_e,
    };
    let mut out = [0u8; 12];
    let _: RecolorStats = ok(read(image.recolor(request(vec![black(10.0)]), &mut out)));
    assert_eq!(out, [0, 0, 0, 0, 230, 76, 60, 255, 0, 0, 0, 0]);

    for bad in [-1.0, 100.5, f32::NAN] {
        assert_eq!(
            error_kind::<RecolorStats>(read(image.recolor(request(vec![black(bad)]), &mut out))),
            ErrorKind::InvalidInput,
            "deltaE {bad}"
        );
    }
    assert_eq!(
        error_kind::<RecolorStats>(read(
            image.recolor(request(vec![black(1.0); 257]), &mut out)
        )),
        ErrorKind::TooManyRanges
    );
}

#[wasm_bindgen_test]
fn unprinted_colors_match_what_recolor_leaves_transparent() {
    let black = MaterialRange {
        pixel: Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        delta_e: 10.0,
    };
    let check = |colors: Vec<Rgb>, material: [u8; 3], ranges: Vec<MaterialRange>| {
        read::<UnprintedColors>(unprinted_colors(
            Ts::from_rust(&UnprintedCheck {
                colors,
                material: rgb(material),
                material_ranges: ranges,
            })
            .unwrap(),
        ))
    };
    // Black and a near-black are within ΔE 10 of black; red is not.
    let colors = vec![rgb([0, 0, 0]), rgb([10, 10, 10]), rgb([230, 76, 60])];
    let flags = ok(check(colors.clone(), [255, 255, 255], vec![black]));
    assert_eq!(flags.unprinted, [true, true, false]);
    // The same as recolor: a one-pixel image of each color is transparent exactly when flagged.
    for (color, flag) in colors.iter().zip(&flags.unprinted) {
        let image = SourceImage::from_rgba(vec![color.r, color.g, color.b, 255], 1, 1).unwrap();
        let mut out = [0u8; 4];
        let request = Ts::from_rust(&RecolorRequest {
            mappings: vec![],
            material: rgb([255, 255, 255]),
            material_ranges: vec![black],
        })
        .unwrap();
        let _: RecolorStats = ok(read(image.recolor(request, &mut out)));
        assert_eq!(out[3] == 0, *flag, "{color:?}");
    }
    // No ranges: nothing is unprinted. Limits and ΔE are checked as for recolor.
    assert_eq!(
        ok(check(colors.clone(), [255, 255, 255], vec![])).unprinted,
        [false; 3]
    );
    assert_eq!(
        error_kind(check(colors.clone(), [255, 255, 255], vec![black; 257])),
        ErrorKind::TooManyRanges
    );
    let bad = MaterialRange {
        delta_e: 101.0,
        ..black
    };
    assert_eq!(
        error_kind(check(colors, [255, 255, 255], vec![bad])),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        error_kind(check(
            vec![rgb([0, 0, 0]); 257],
            [255, 255, 255],
            vec![black]
        )),
        ErrorKind::TooManyMappings
    );
}

#[wasm_bindgen_test]
fn configs_carry_the_unprinted_colors() {
    // Absent: none.
    let parsed: ParsedConfig = ok(read(parse_config("[[palette]]\nsize = 1\npicks = []\n")));
    assert!(parsed.unprinted.is_empty());

    let text = "material = [0, 0, 0]\nunprinted = [\n\
                { material = true, delta_e = 10 },\n\
                { rgba = [200, 40, 40, 255], delta_e = 12.5 },\n]\n\n\
                [[palette]]\nsize = 1\npicks = []\n";
    let parsed: ParsedConfig = ok(read(parse_config(text)));
    let expected = vec![
        ConfigUnprinted::Material { delta_e: 10.0 },
        ConfigUnprinted::Color {
            rgba: Rgba {
                r: 200,
                g: 40,
                b: 40,
                a: 255,
            },
            delta_e: 12.5,
        },
    ];
    assert_eq!(parsed.unprinted, expected);
    // The TypeScript shape: a tagged union with camelCase fields.
    let js = Ts::from_rust(&parsed.unprinted[1]).unwrap().js_value();
    assert_eq!(property(&js, "kind"), "color");
    assert_eq!(property(&js, "deltaE"), 12.5);

    // Exported again, always written; out-of-range ΔE is an invalidConfig error.
    let export = |unprinted: Vec<ConfigUnprinted>| ConfigExport {
        image_name: "x.png".into(),
        material: rgb([0, 0, 0]),
        unprinted,
        picks: vec![],
    };
    let out: ConfigText = ok(read(serialize_config(
        Ts::from_rust(&export(expected.clone())).unwrap(),
    )));
    let again: ParsedConfig = ok(read(parse_config(&out.text)));
    assert_eq!(again.unprinted, expected);
    let none: ConfigText = ok(read(serialize_config(
        Ts::from_rust(&export(vec![])).unwrap(),
    )));
    assert!(none.text.contains("\nunprinted = []\n"), "{}", none.text);
    let bad = export(vec![ConfigUnprinted::Material { delta_e: 101.0 }]);
    assert_eq!(
        error_kind::<ConfigText>(read(serialize_config(Ts::from_rust(&bad).unwrap()))),
        ErrorKind::InvalidConfig
    );
}
