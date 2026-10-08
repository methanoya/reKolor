//! Runs inside WASM: `wasm-pack test --node crates/wasm` (X3 b).
//!
//! Proves the interface on the WASM target: the baseline (R9) reproduced inside WASM (native
//! and WASM float math agree), malformed input coming back as error values while the module keeps
//! working, and buffer-length checks. R11: the ΔE fingerprint, the Sharma reference data and the
//! Pantone suggestion snapshot, all checked against the same files as the native tests.

#![cfg(target_arch = "wasm32")]

#[path = "../../../testdata/generator.rs"]
mod generator;

use rekolor_core::Rgb8;
use rekolor_wasm::{
    ConfigExport, ConfigPick, ConfigSection, ConfigText, ErrorKind, ImageStats, Mapping, Outcome,
    Palette, PaletteData, PaletteEntry, PaletteMatch, PaletteMatches, ParsedConfig, Pick,
    RecolorRequest, RecolorStats, ResolvedPicks, Rgb, Rgba, SourceImage, parse_config,
    serialize_config,
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

#[wasm_bindgen_test]
fn zero_and_one_mapping_follow_the_contract() {
    // I8: no mappings → the composited copy; one mapping → every pixel takes that ink.
    let image = SourceImage::from_rgba(vec![230, 76, 60, 255, 9, 9, 9, 0], 2, 1).unwrap();
    let mut out = [0u8; 8];

    let none = Ts::from_rust(&RecolorRequest { mappings: vec![] }).unwrap();
    let stats: RecolorStats = ok(read(image.recolor(none, &mut out)));
    assert_eq!(out, [230, 76, 60, 255, 255, 255, 255, 255]);
    assert_eq!((stats.exact, stats.nearest), (0.0, 2.0));

    let one = Ts::from_rust(&RecolorRequest {
        mappings: vec![Mapping {
            source: rgb([230, 76, 60]),
            ink: rgb([40, 120, 200]),
        }],
    })
    .unwrap();
    let stats: RecolorStats = ok(read(image.recolor(one, &mut out)));
    assert_eq!(out, [40, 120, 200, 255, 40, 120, 200, 255]);
    assert_eq!((stats.exact, stats.nearest), (1.0, 1.0));
}

#[wasm_bindgen_test]
fn delta_e_matches_the_native_fingerprint() {
    // R11: bit-identical ΔE on native and WASM. The file was recorded natively.
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
    // I9 in WASM: the exact tie from the core test (same bits on both targets since R11).
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
            mappings: vec![
                Mapping {
                    source: rgb([230, 76, 60]),
                    ink: first,
                },
                Mapping {
                    source: rgb([40, 120, 200]),
                    ink: second,
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
    include_str!("../../../../samples/good-looking/04-tiger.palettes.toml"),
    include_str!("../../../../samples/arbitrary/07-alpha-hue.palettes.toml"),
    include_str!("../../../../samples/others/icon-calendar.palettes.toml"),
];

#[wasm_bindgen_test]
fn sample_configs_parse_resolve_and_round_trip() {
    // W10 v: the app reads the golden-set configs exactly like the CLI.
    let palette = pantone_palette();
    for text in SAMPLE_CONFIGS {
        let parsed: ParsedConfig = ok(read(parse_config(text)));
        assert_eq!(
            parsed.sections.iter().map(|s| s.size).collect::<Vec<_>>(),
            [3, 7, 16]
        );
        for section in &parsed.sections {
            let resolved: ResolvedPicks = ok(read(
                palette.resolve_section(Ts::from_rust(section).unwrap()),
            ));
            assert_eq!(resolved.picks.len(), section.picks.len());
            for (pick, r) in section.picks.iter().zip(&resolved.picks) {
                assert_eq!(r.pixel, pick.rgba);
                assert_eq!(r.ink.name, pick.ink);
            }
            // Export (one section, size = number of picks) and read it back.
            let export = ConfigExport {
                image_name: "x.png".into(),
                picks: section.picks.clone(),
            };
            let text: ConfigText = ok(read(serialize_config(Ts::from_rust(&export).unwrap())));
            assert!(
                text.text
                    .starts_with("# Palette exported from the reKolor web app for x.png.")
            );
            let again: ParsedConfig = ok(read(parse_config(&text.text)));
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
            },
            ConfigPick {
                rgba: Rgba {
                    r: 203,
                    g: 0,
                    b: 0,
                    a: 100,
                },
                ink: "Pantone 185".into(),
            },
        ],
    };
    let resolved: ResolvedPicks = ok(read(
        palette.resolve_section(Ts::from_rust(&section).unwrap()),
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
        }],
    };
    assert_eq!(
        error_kind::<ResolvedPicks>(read(
            palette.resolve_section(Ts::from_rust(&unknown).unwrap())
        )),
        ErrorKind::InvalidConfig
    );
}
