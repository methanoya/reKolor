//! Deterministic test inputs for the baseline and the generated-buffer tests.
//!
//! Plain Rust with no dependencies and no crate-level attributes, so it can be included as a
//! module from anywhere (`#[path = ".../testdata/generator.rs"] mod generator;`): the baseline
//! recorder, native tests and WASM tests all use the identical data.
//!
//! Changing anything here changes the inputs, so the baseline in `testdata/baseline/` must be
//! re-recorded in the same change.

// Each program that includes this file uses only some of its functions; `allow(dead_code)`
// silences the compiler's "never used" warnings for the rest. (`#!` applies an attribute to the
// enclosing module, here the whole file.)
#![allow(dead_code)]

/// An RGBA8 image: row-major, 4 bytes per pixel, straight (not premultiplied) alpha.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Picked colors and their inks (`from` → `to`).
#[derive(Debug, Clone, Copy)]
pub struct MappingSet {
    pub name: &'static str,
    pub pairs: &'static [([u8; 3], [u8; 3])],
}

// The mapping sets every fixture is recolored with; the baseline has one PNG per pair.
pub const MAPPING_SETS: &[MappingSet] = &[
    // No picks at all.
    MappingSet {
        name: "none",
        pairs: &[],
    },
    // A single pick: every pixel ends up with one ink.
    MappingSet {
        name: "one",
        pairs: &[([255, 184, 0], [252, 181, 20])], // Pantone 1235
    },
    // Three picks: red, a pale gray and white.
    MappingSet {
        name: "calendar3",
        pairs: &[
            ([230, 76, 60], [226, 61, 40]),     // Pantone 179
            ([235, 239, 240], [214, 219, 224]), // Pantone 656
            ([255, 255, 255], [255, 255, 255]), // Pure White (non-palette)
        ],
    },
    // Colors from the README screenshot plus white and black, each with a nearby Pantone ink.
    MappingSet {
        name: "screenshot8",
        pairs: &[
            ([255, 184, 0], [252, 181, 20]),    // Pantone 1235
            ([242, 237, 207], [224, 221, 188]), // Pantone 5807
            ([44, 44, 57], [53, 56, 66]),       // Pantone 532
            ([241, 245, 107], [232, 237, 96]),  // Pantone 386
            ([193, 205, 35], [186, 196, 5]),    // Pantone 390
            ([230, 169, 142], [239, 181, 160]), // Pantone 487
            ([255, 255, 255], [255, 255, 255]), // Pure White (non-palette)
            ([0, 0, 0], [0, 0, 0]),             // Pure Black (non-palette)
        ],
    },
    // Inks deliberately far from their sources (each source gets another source's ink), so
    // matching against inks and matching against picked colors give visibly different results.
    MappingSet {
        name: "swapped4",
        pairs: &[
            ([230, 76, 60], [58, 117, 196]), // red → Pantone 285 (blue)
            ([40, 120, 200], [226, 61, 40]), // blue → Pantone 179 (red)
            ([255, 184, 0], [53, 56, 66]),   // yellow → Pantone 532 (dark)
            ([44, 44, 57], [252, 181, 20]),  // dark → Pantone 1235 (yellow)
        ],
    },
];

pub fn fixtures() -> Vec<Fixture> {
    vec![
        alpha_ramp(),
        edges(),
        picks(),
        gradient(),
        noise(),
        transparent(),
    ]
}

// Look up a set or a fixture by name; an unknown name is a bug in the test, so it panics.
pub fn mapping_set(name: &str) -> MappingSet {
    *MAPPING_SETS
        .iter()
        .find(|m| m.name == name)
        .unwrap_or_else(|| panic!("unknown mapping set {name}"))
}

pub fn fixture(name: &str) -> Fixture {
    fixtures()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("unknown fixture {name}"))
}

// Builds a fixture by calling `pixel(x, y)` for every position, row by row.
// `impl Fn(u32, u32) -> [u8; 4]` accepts any function or closure with that signature.
fn image(
    name: &'static str,
    width: u32,
    height: u32,
    pixel: impl Fn(u32, u32) -> [u8; 4],
) -> Fixture {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&pixel(x, y));
        }
    }
    Fixture {
        name,
        width,
        height,
        rgba,
    }
}

/// Every alpha value 0..=255 (one column each) on several colors, including fully transparent.
fn alpha_ramp() -> Fixture {
    // A constant inside a function: an array of eight RGB colors (`[[u8; 3]; 8]`).
    const ROWS: [[u8; 3]; 8] = [
        [230, 76, 60],
        [255, 184, 0],
        [44, 44, 57],
        [0, 0, 0],
        [255, 255, 255],
        [40, 120, 200],
        [0, 140, 130],
        [193, 205, 35],
    ];
    image("alpha_ramp", 256, ROWS.len() as u32, |x, y| {
        let [r, g, b] = ROWS[y as usize];
        [r, g, b, x as u8]
    })
}

/// Print-like artwork on a transparent background: a disc with an alpha-anti-aliased rim, an
/// opaque color-anti-aliased edge (blend between two colors), and a solid dark band.
fn edges() -> Fixture {
    const YELLOW: [u8; 3] = [255, 184, 0];
    const DARK: [u8; 3] = [44, 44, 57];
    const RED: [u8; 3] = [230, 76, 60];
    let mix = |a: [u8; 3], b: [u8; 3], t: u32| -> [u8; 3] {
        // t in 0..=8: integer interpolation, identical on every target
        let ch = |i: usize| ((u32::from(a[i]) * (8 - t) + u32::from(b[i]) * t) / 8) as u8;
        [ch(0), ch(1), ch(2)]
    };
    image("edges", 96, 96, |x, y| {
        let (x, y) = (x as i32, y as i32);
        // Squared distance from the disc's center (40, 40); comparing squares avoids a square root.
        let d2 = (x - 40).pow(2) + (y - 40).pow(2);
        if (70..76).contains(&y) {
            let [r, g, b] = DARK;
            [r, g, b, 255]
        } else if (78..96).contains(&y) {
            // horizontal opaque blend from RED to YELLOW over 9 columns starting at x = 40
            let t = (x - 40).clamp(0, 8) as u32;
            let [r, g, b] = mix(RED, YELLOW, t);
            [r, g, b, 255]
        } else if d2 <= 26 * 26 {
            let [r, g, b] = YELLOW;
            [r, g, b, 255]
        } else if d2 <= 27 * 27 {
            let [r, g, b] = YELLOW;
            [r, g, b, 191]
        } else if d2 <= 28 * 28 {
            let [r, g, b] = YELLOW;
            [r, g, b, 127]
        } else if d2 <= 29 * 29 {
            let [r, g, b] = YELLOW;
            [r, g, b, 64]
        } else {
            [255, 255, 255, 0]
        }
    })
}

/// Exact pick colors and their one-step neighbors: one row per distinct source color used in
/// `MAPPING_SETS`, columns = exact, r+1, r-1, g+1, g-1, b+1, b-1 (saturating). Opaque.
fn picks() -> Fixture {
    let mut sources: Vec<[u8; 3]> = Vec::new();
    for set in MAPPING_SETS {
        for &(source, _) in set.pairs {
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
    }
    const STEPS: [(usize, i16); 7] = [(0, 0), (0, 1), (0, -1), (1, 1), (1, -1), (2, 1), (2, -1)];
    image("picks", STEPS.len() as u32, sources.len() as u32, |x, y| {
        let mut c = sources[y as usize];
        let (channel, delta) = STEPS[x as usize];
        c[channel] = (i16::from(c[channel]) + delta).clamp(0, 255) as u8;
        [c[0], c[1], c[2], 255]
    })
}

/// Smooth opaque gradients: many unique colors.
fn gradient() -> Fixture {
    image("gradient", 64, 48, |x, y| {
        [(x * 4) as u8, (y * 5) as u8, (255 - x * 2) as u8, 255]
    })
}

/// High-entropy pixels: random RGB, alpha drawn from values around the interesting edges.
fn noise() -> Fixture {
    const ALPHAS: [u8; 8] = [0, 1, 64, 127, 128, 200, 255, 255];
    let (width, height) = (128u32, 96u32);
    // A tiny pseudo-random generator (xorshift): the same seed always gives the same "random"
    // pixels, on every machine and target. `_` in a number literal is only a digit separator.
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width * height {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        rgba.extend_from_slice(&[
            state as u8,
            (state >> 8) as u8,
            (state >> 16) as u8,
            ALPHAS[((state >> 24) % ALPHAS.len() as u64) as usize],
        ]);
    }
    Fixture {
        name: "noise",
        width,
        height,
        rgba,
    }
}

/// Colors for the Pantone suggestion snapshot: a 16-level grid (0, 17, …, 255 per channel; 4,096
/// colors) plus 1,000 further distinct colors off the grid, from a SplitMix64 sequence using its
/// high bits.
pub fn suggestion_colors() -> Vec<[u8; 3]> {
    let mut colors: Vec<[u8; 3]> = Vec::with_capacity(5096);
    for r in (0..=255u16).step_by(17) {
        for g in (0..=255u16).step_by(17) {
            for b in (0..=255u16).step_by(17) {
                colors.push([r as u8, g as u8, b as u8]);
            }
        }
    }
    let mut seen: std::collections::HashSet<[u8; 3]> = colors.iter().copied().collect();
    let mut state: u64 = 0x5eed_c0de_0000_0001;
    while colors.len() < 4096 + 1000 {
        // SplitMix64, a well-known simple generator; `wrapping_*` arithmetic wraps around on
        // overflow instead of panicking (which debug builds would do for plain `+` and `*`).
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        let color = [(z >> 56) as u8, (z >> 48) as u8, (z >> 40) as u8];
        if seen.insert(color) {
            colors.push(color);
        }
    }
    colors
}

/// Fully transparent pixels with different hidden RGB values.
fn transparent() -> Fixture {
    image("transparent", 16, 16, |x, y| {
        [(x * 16) as u8, (y * 16) as u8, 128, 0]
    })
}

// The ΔE fingerprint: ΔE for 100,000 generated color pairs, reduced to one hash. Floating-point
// math can differ slightly between compilers and CPUs; matching the recorded hash proves the native
// and WASM builds compute every ΔE to the exact same bits, so ties between inks resolve the same
// way in the CLI and in the browser.
/// Seed of the ΔE fingerprint corpus.
pub const FINGERPRINT_SEED: u64 = 0x5eed;
/// Number of color pairs in the ΔE fingerprint.
pub const FINGERPRINT_PAIRS: usize = 100_000;

/// Pairs whose ΔE is recorded exactly (as `f32` bits) beside the fingerprint hash.
pub const FINGERPRINT_EXACT: &[(&str, [u8; 3], [u8; 3])] = &[
    ("black-white", [0, 0, 0], [255, 255, 255]),
    ("red-blue", [230, 76, 60], [58, 117, 196]),
    // The exact tie used by the earlier-mapping tests (core, WASM, TS contract): same bits.
    ("tie-a", [128, 128, 128], [0, 2, 227]),
    ("tie-b", [128, 128, 128], [0, 4, 0]),
    // A pair that tied exactly with the `lab` + `deltae` crates natively, but not in WASM.
    ("old-tie-a", [128, 128, 128], [100, 140, 117]),
    ("old-tie-b", [128, 128, 128], [101, 101, 135]),
];

/// The ΔE fingerprint file (`testdata/baseline/delta-e-fingerprint.tsv`), rendered from a ΔE
/// function. The update command writes it; the native and WASM tests render it again with their
/// own build and compare the text, which proves bit-identical ΔE on both targets.
///
/// Corpus: SplitMix64 from [`FINGERPRINT_SEED`]; each 64-bit word gives one pair, from its
/// little-endian bytes 0–2 and 3–5; [`FINGERPRINT_PAIRS`] pairs in generation order.
/// Hash: FNV-1a 64 (offset `0xcbf29ce484222325`, prime `0x100000001b3`) over each `f32::to_bits()`
/// as little-endian bytes.
pub fn delta_e_fingerprint(delta_e: impl Fn([u8; 3], [u8; 3]) -> f32) -> String {
    let mut state = FINGERPRINT_SEED;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for _ in 0..FINGERPRINT_PAIRS {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        // The 64-bit word as 8 bytes (little-endian); bytes 0–2 and 3–5 become the two colors.
        let b = z.to_le_bytes();
        for byte in delta_e([b[0], b[1], b[2]], [b[3], b[4], b[5]])
            .to_bits()
            .to_le_bytes()
        {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    // A trailing `\` in a string literal continues it on the next line, skipping the indentation.
    let mut out = format!(
        "# ΔE fingerprint, format 1: {FINGERPRINT_PAIRS} SplitMix64 pairs from seed \
         {FINGERPRINT_SEED:#x}, FNV-1a 64 over f32 bits (see generator.rs)\n\
         hash\t{hash:#018x}\n\
         # name\tfrom\tto\tdelta_e_bits\tdelta_e\n"
    );
    for &(name, from, to) in FINGERPRINT_EXACT {
        let d = delta_e(from, to);
        let rgb = |c: [u8; 3]| format!("{},{},{}", c[0], c[1], c[2]);
        out.push_str(&format!(
            "{name}\t{}\t{}\t{:#010x}\t{d}\n",
            rgb(from),
            rgb(to),
            d.to_bits()
        ));
    }
    out
}
