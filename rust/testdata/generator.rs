//! Deterministic test inputs for the baseline (R9) and the generated-buffer tests (X1).
//!
//! Plain Rust with no dependencies and no crate-level attributes, so it can be included as a
//! module from anywhere (`#[path = ".../testdata/generator.rs"] mod generator;`): the baseline
//! recorder, native tests and WASM tests all use the identical data.
//!
//! Changing anything here changes the inputs, so the baseline in `testdata/baseline/` must be
//! re-recorded in the same change.

#![allow(dead_code)]

/// An RGBA8 image: row-major, 4 bytes per pixel, straight (not premultiplied) alpha.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Picked colors and their inks, as the 2023 UI sent them (`from` → `to`).
#[derive(Debug, Clone, Copy)]
pub struct MappingSet {
    pub name: &'static str,
    pub pairs: &'static [([u8; 3], [u8; 3])],
}

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
    // The three pairs from the 2023 `main.rs` harness.
    MappingSet {
        name: "calendar3",
        pairs: &[
            ([230, 76, 60], [226, 61, 40]),     // Pantone 179
            ([235, 239, 240], [214, 219, 224]), // Pantone 656
            ([255, 255, 255], [255, 255, 255]), // Pure White (non-palette)
        ],
    },
    // Colors from the README screenshot plus white and black, each with the ink the 2023 JS
    // suggested for it.
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

/// Fully transparent pixels with different hidden RGB values.
fn transparent() -> Fixture {
    image("transparent", 16, 16, |x, y| {
        [(x * 16) as u8, (y * 16) as u8, 128, 0]
    })
}
