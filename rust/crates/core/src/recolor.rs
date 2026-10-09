// Recoloring: every pixel of the image becomes one of the chosen inks (or stays unprinted). This is
// the heart of the engine; the web app calls it on every change through `rekolor-wasm`, and the CLI
// calls it to write PNG files.
use crate::color::{Lab, delta_e_2000_lab, lab};
use crate::{Error, ImageRef, Rgb8, Rgba8, composite};

// A pick as the engine sees it: the color the user clicked (composited over the material), the
// color of the ink chosen for it, and how far around that color the pick reaches.
/// One picked color and the ink that replaces it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapping {
    pub source: Rgb8,
    pub ink: Rgb8,
    /// The pick's capture radius (CIEDE2000, 0 to 100): pixels within it of `source` take this
    /// ink before the nearest-ink rule applies. 0: only an exact match (see [`recolor`]).
    pub delta_e: f32,
}

/// A color left unprinted: pixels whose composited color is within `delta_e`
/// (CIEDE2000) of `pixel` composited over the material take no ink, so the material shows there.
/// Saves ink where the material already has the color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialRange {
    /// A pixel from the image (or the material itself, opaque), as stored; composited over the
    /// material on each call, so a range follows a material change.
    pub pixel: Rgba8,
    pub delta_e: f32,
}

// Counts of how each pixel was decided, for logs and tests. `Default` gives all zeros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecolorStats {
    /// Pixels whose composited color equals a mapping's source exactly.
    pub exact: u64,
    /// Pixels within a mapping's capture radius (`delta_e`) of its source, but not equal to it.
    pub captured: u64,
    /// Pixels left unprinted by a [`MaterialRange`] (transparent in the output).
    pub unprinted: u64,
    /// All other pixels.
    pub nearest: u64,
}

// `&[Mapping]` borrows a list of mappings; `&mut [u8]` borrows the output buffer for writing (only
// one `&mut` borrow can exist at a time, so nothing else can touch `out` meanwhile).
/// Recolors `image` into `out`, which must have the same length as the source buffer.
///
/// Contract ([`Rgb8::WHITE`] as the material composites over white):
/// 1. each pixel is composited over `material` ([`composite`]); the output is opaque;
/// 2. if the result equals a mapping's `source`, it takes that mapping's ink (first match wins);
/// 3. otherwise, if it is within one or more mappings' capture radius (`delta_e`, CIEDE2000, from
///    `source`), it takes the ink of the mapping whose source is nearest (on a tie, the earlier);
/// 4. otherwise it takes the **ink** nearest to it by CIEDE2000 (on a tie, the earlier mapping);
/// 5. **no mappings**: the output is the composited copy (the image as it looks on the material);
/// 6. **one mapping**: no special case, so every pixel takes that ink.
///
/// With every `delta_e` at 0, step 3 never applies.
///
/// ΔE is bit-identical on native and WASM builds ([`delta_e_2000_lab`](crate::delta_e_2000_lab)),
/// so ties resolve the same way everywhere.
pub fn recolor(
    image: ImageRef<'_>,
    mappings: &[Mapping],
    material: Rgb8,
    out: &mut [u8],
) -> Result<RecolorStats, Error> {
    recolor_with_ranges(image, mappings, material, &[], out)
}

/// [`recolor`], plus colors left unprinted: a pixel within a range's ΔE
/// takes no ink and is **transparent** in the output (`[0, 0, 0, 0]`), so the material shows
/// through. Ranges are checked first, so they win over an exact pick. With no ranges this is
/// [`recolor`] exactly.
pub fn recolor_with_ranges(
    image: ImageRef<'_>,
    mappings: &[Mapping],
    material: Rgb8,
    ranges: &[MaterialRange],
    out: &mut [u8],
) -> Result<RecolorStats, Error> {
    // The output has the same layout as the input: 4 bytes per pixel.
    let input = image.as_bytes();
    if out.len() != input.len() {
        return Err(Error::OutputLength {
            expected: input.len(),
            actual: out.len(),
        });
    }

    // Convert every ink, every capture source and every range color to Lab once, up front: the
    // loop below compares each pixel against them, and converting inside the loop would repeat
    // that work millions of times. `(Mapping, Lab)` is a tuple, an unnamed pair of values.
    let inks: Vec<(Mapping, Lab)> = mappings.iter().map(|&m| (m, lab(m.ink))).collect();
    let radii = radius_labs(mappings);
    let ranges = range_labs(ranges, material);
    let table = Lookup {
        inks: &inks,
        radii: &radii,
    };
    let pixel_count = input.len() / 4;
    // The memo (a cache of answers per color) only pays off for larger images; `.then(|| ...)`
    // builds it (`Some(table)`) only when the condition is true, and leaves `None` otherwise.
    let mut memo =
        (pixel_count > MEMO_MIN_PIXELS && !inks.is_empty() && inks.len() < MEMO_MAX_INKS)
            .then(|| vec![0u16; 1 << 24]);
    let mut stats = RecolorStats::default();
    // Walk input pixels and output pixels side by side: `zip` pairs them up, and
    // `as_chunks_mut::<4>` views the output bytes as 4-byte pixels that can be written in place.
    for (pixel, dst) in image.pixels().zip(out.as_chunks_mut::<4>().0) {
        let color = composite(pixel, material);
        // `match` picks the first arm whose pattern fits (here: memo or no memo, and for no memo
        // whether the color is in an unprinted range). `None if ... =>` is an arm with an extra
        // condition.
        let decided = match memo.as_deref_mut() {
            Some(memo) => memoized(memo, &table, &ranges, color),
            None if in_range(&ranges, color) => Decided::Unprinted,
            None => {
                let (ink, how) = table.ink(color);
                Decided::Ink { ink, how }
            }
        };
        match decided {
            Decided::Ink { ink, how } => {
                // `*dst = ...` writes through the mutable reference into the output buffer; 255 =
                // fully opaque.
                *dst = [ink.r, ink.g, ink.b, 255];
                match how {
                    How::Exact => stats.exact += 1,
                    How::Captured => stats.captured += 1,
                    How::Nearest => stats.nearest += 1,
                }
            }
            Decided::Unprinted => {
                *dst = [0, 0, 0, 0];
                stats.unprinted += 1;
            }
        }
    }
    // Shown only when debug logging is on (e.g. `RUST_LOG=debug` for the CLI).
    log::debug!(
        "recolor: {}×{} on {:?}, {} mappings, {} ranges, exact {}, captured {}, unprinted {}, \
         nearest {}",
        image.width(),
        image.height(),
        material,
        mappings.len(),
        ranges.len(),
        stats.exact,
        stats.captured,
        stats.unprinted,
        stats.nearest
    );
    Ok(stats)
}

// `1 << 16` is 2^16 = 65,536 (a left shift doubles the number per step).
/// Images above this many pixels remember each color's answer (below it, the 32 MiB table costs
/// more than it saves).
const MEMO_MIN_PIXELS: usize = 1 << 16;
// A memo entry is one `u16`. Its low 14 bits (`MEMO_INDEX`) are 0 (not computed yet), 0x3fff
// (`MEMO_UNPRINTED`: in an unprinted range) or "mapping index + 1"; the top bit (`0x8000`) marks an
// exact match and the next one (`0x4000`) a capture-radius match. So the memo is used only with
// fewer than 0x3fff mappings (the WASM API and configs allow 256), which keeps 0x3fff free for
// "unprinted".
/// The memo stores `mapping index + 1` in 14 bits.
const MEMO_MAX_INKS: usize = 0x3fff;
const MEMO_INDEX: u16 = 0x3fff;
const MEMO_EXACT: u16 = 0x8000;
const MEMO_CAPTURED: u16 = 0x4000;
/// Unprinted: `0x3fff` is never `index + 1` (at most `MEMO_MAX_INKS - 1` mappings).
const MEMO_UNPRINTED: u16 = 0x3fff;

// Private helper types (no `pub`): the per-pixel answer used inside this file.
/// How a composited color found its ink (for [`RecolorStats`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum How {
    /// Equal to a mapping's source.
    Exact,
    /// Within a mapping's capture radius.
    Captured,
    /// The nearest ink (or, with no mappings, the composited color itself).
    Nearest,
}

/// What a composited color becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decided {
    /// An ink (or, with no mappings, the composited color itself), and how it was found.
    Ink { ink: Rgb8, how: How },
    /// Within a material range: no ink.
    Unprinted,
}

/// Whether [`recolor_with_ranges`] leaves a composited color unprinted on `material`: the same
/// test it applies to every pixel. `color` is a color as recolor matches it (already composited
/// over the material, like a pick's matching color).
pub fn is_unprinted(color: Rgb8, material: Rgb8, ranges: &[MaterialRange]) -> bool {
    in_range(&range_labs(ranges, material), color)
}

/// Each range's center (its pixel composited over `material`) in Lab, with its ΔE.
fn range_labs(ranges: &[MaterialRange], material: Rgb8) -> Vec<(Lab, f32)> {
    ranges
        .iter()
        .map(|r| (lab(composite(r.pixel, material)), r.delta_e))
        .collect()
}

// `any` is true as soon as one range contains the color (it stops at the first).
/// Whether a composited color is within a material range.
fn in_range(ranges: &[(Lab, f32)], color: Rgb8) -> bool {
    if ranges.is_empty() {
        return false;
    }
    let color_lab = lab(color);
    ranges
        .iter()
        .any(|&(center, delta_e)| delta_e_2000_lab(color_lab, center) <= delta_e)
}

/// [`in_range`] and [`Lookup::index`] remembered per composited color (one `u16` for each of the
/// 2^24 colors: 0 = not yet computed, [`MEMO_UNPRINTED`] = in a material range, else the mapping
/// index + 1, with [`MEMO_EXACT`] or [`MEMO_CAPTURED`] for how it matched). The answer depends
/// only on the color, so results are identical; images repeat colors, so most pixels skip the
/// CIEDE2000 work. Memory is fixed (32 MiB) whatever the image.
fn memoized(memo: &mut [u16], table: &Lookup<'_>, ranges: &[(Lab, f32)], color: Rgb8) -> Decided {
    // The color as one number 0..2^24, used as the index into the memo table.
    let key = (usize::from(color.r) << 16) | (usize::from(color.g) << 8) | usize::from(color.b);
    let entry = memo[key];
    if entry == MEMO_UNPRINTED {
        return Decided::Unprinted;
    }
    // Already computed: decode the stored index (`entry & MEMO_INDEX` keeps the low 14 bits) and
    // how it matched (the two top bits).
    if entry != 0 {
        let how = if entry & MEMO_EXACT != 0 {
            How::Exact
        } else if entry & MEMO_CAPTURED != 0 {
            How::Captured
        } else {
            How::Nearest
        };
        return Decided::Ink {
            ink: table.inks[usize::from(entry & MEMO_INDEX) - 1].0.ink,
            how,
        };
    }
    if in_range(ranges, color) {
        memo[key] = MEMO_UNPRINTED;
        return Decided::Unprinted;
    }
    let (index, how) = table.index(color);
    // `inks` is non-empty when the memo is used, so `index` always finds one.
    let index = index.expect("memo is only used with mappings");
    let flag = match how {
        How::Exact => MEMO_EXACT,
        How::Captured => MEMO_CAPTURED,
        How::Nearest => 0,
    };
    memo[key] = (index as u16 + 1) | flag;
    Decided::Ink {
        ink: table.inks[index].0.ink,
        how,
    }
}

/// The mappings with a capture radius: each one's index, its source in Lab, and its radius. Only
/// these are checked in step 3 of [`recolor`]; with no radius set, the list is empty and costs
/// nothing.
fn radius_labs(mappings: &[Mapping]) -> Vec<(usize, Lab, f32)> {
    mappings
        .iter()
        .enumerate()
        .filter(|(_, m)| m.delta_e > 0.0)
        .map(|(index, m)| (index, lab(m.source), m.delta_e))
        .collect()
}

// `'a` is a lifetime: the struct borrows the two lists for as long as it exists.
/// The mappings prepared for matching: every mapping with its ink in Lab, and the ones with a
/// capture radius ([`radius_labs`]).
struct Lookup<'a> {
    inks: &'a [(Mapping, Lab)],
    radii: &'a [(usize, Lab, f32)],
}

impl Lookup<'_> {
    // Returns a tuple: the ink color and how it was found.
    /// The ink for one composited pixel color, and how it was found.
    fn ink(&self, color: Rgb8) -> (Rgb8, How) {
        match self.index(color) {
            (Some(index), how) => (self.inks[index].0.ink, how),
            // No mappings: the composited pixel passes through.
            (None, how) => (color, how),
        }
    }

    /// Which mapping a composited color takes: an exact source match first (first match wins),
    /// then the nearest source among the mappings whose capture radius contains the color (ties:
    /// the earlier mapping), else the nearest ink (ties: the earlier mapping). `None` only when
    /// there are no mappings.
    fn index(&self, color: Rgb8) -> (Option<usize>, How) {
        // `position` gives the index of the first mapping whose source is exactly this color.
        if let Some(index) = self.inks.iter().position(|(m, _)| m.source == color) {
            return (Some(index), How::Exact);
        }
        // The pixel's Lab value is computed once instead of once per mapping (same value each
        // time).
        let color_lab = lab(color);
        let mut captured: Option<(usize, f32)> = None;
        for &(index, source_lab, delta_e) in self.radii {
            let distance = delta_e_2000_lab(color_lab, source_lab);
            if distance <= delta_e && captured.is_none_or(|(_, d)| d > distance) {
                captured = Some((index, distance));
            }
        }
        if let Some((index, _)) = captured {
            return (Some(index), How::Captured);
        }
        let mut best: Option<(usize, f32)> = None;
        for (index, &(_, ink_lab)) in self.inks.iter().enumerate() {
            let distance = delta_e_2000_lab(color_lab, ink_lab);
            if best.is_none_or(|(_, d)| d > distance) {
                best = Some((index, distance));
            }
        }
        (best.map(|(index, _)| index), How::Nearest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb8 = Rgb8::new(230, 76, 60);
    const BLUE: Rgb8 = Rgb8::new(40, 120, 200);

    const BLACK: Rgb8 = Rgb8::new(0, 0, 0);

    fn run(rgba: &[u8], mappings: &[Mapping]) -> (Vec<u8>, RecolorStats) {
        run_on(rgba, mappings, Rgb8::WHITE)
    }

    fn run_on(rgba: &[u8], mappings: &[Mapping], material: Rgb8) -> (Vec<u8>, RecolorStats) {
        let image = ImageRef::new(rgba, (rgba.len() / 4) as u32, 1).unwrap();
        let mut out = vec![0; rgba.len()];
        let stats = recolor(image, mappings, material, &mut out).unwrap();
        (out, stats)
    }

    #[test]
    fn rejects_output_of_wrong_length() {
        let image = ImageRef::new(&[0; 8], 2, 1).unwrap();
        assert_eq!(
            recolor(image, &[], Rgb8::WHITE, &mut [0; 4]).unwrap_err(),
            Error::OutputLength {
                expected: 8,
                actual: 4
            }
        );
    }

    #[test]
    fn exact_source_match_takes_its_ink() {
        let mappings = [Mapping {
            source: RED,
            ink: BLUE,
            delta_e: 0.0,
        }];
        let (out, stats) = run(&[230, 76, 60, 255], &mappings);
        assert_eq!(out, [40, 120, 200, 255]);
        assert_eq!((stats.exact, stats.nearest), (1, 0));
    }

    #[test]
    fn exact_match_is_checked_after_compositing() {
        // Fully transparent → white, which equals the white source.
        let mappings = [Mapping {
            source: Rgb8::new(255, 255, 255),
            ink: Rgb8::new(1, 2, 3),
            delta_e: 0.0,
        }];
        let (out, stats) = run(&[9, 9, 9, 0], &mappings);
        assert_eq!(out, [1, 2, 3, 255]);
        assert_eq!(stats.exact, 1);
    }

    #[test]
    fn other_pixels_take_the_nearest_ink_not_the_nearest_source() {
        // Next to RED's source, but RED's ink is black and BLUE's ink is almost RED.
        let mappings = [
            Mapping {
                source: RED,
                ink: Rgb8::new(0, 0, 0),
                delta_e: 0.0,
            },
            Mapping {
                source: BLUE,
                ink: Rgb8::new(231, 77, 61),
                delta_e: 0.0,
            },
        ];
        let (out, stats) = run(&[231, 76, 60, 255], &mappings);
        assert_eq!(out, [231, 77, 61, 255]);
        assert_eq!((stats.exact, stats.nearest), (0, 1));
    }

    #[test]
    fn nearest_ink_ties_go_to_the_earlier_mapping() {
        // Two different inks at exactly the same CIEDE2000 distance from gray, 39.605984 (the
        // first exact tie in a search over all sRGB colors in r, g, b order). The same pair is in
        // the ΔE fingerprint, the WASM suite and the TS contract. If the ΔE math ever changes, the
        // precondition fails instead of testing a non-tie.
        let pixel = Rgb8::new(128, 128, 128);
        let a = Rgb8::new(0, 2, 227);
        let b = Rgb8::new(0, 4, 0);
        assert_eq!(
            crate::delta_e_2000(pixel, a).to_bits(),
            crate::delta_e_2000(pixel, b).to_bits(),
            "precondition: an exact tie"
        );
        let first_a = [
            Mapping {
                source: RED,
                ink: a,
                delta_e: 0.0,
            },
            Mapping {
                source: BLUE,
                ink: b,
                delta_e: 0.0,
            },
        ];
        let first_b = [first_a[1], first_a[0]];
        let rgba = [128, 128, 128, 255];
        assert_eq!(run(&rgba, &first_a).0, [a.r, a.g, a.b, 255]);
        assert_eq!(run(&rgba, &first_b).0, [b.r, b.g, b.b, 255]);
    }

    #[test]
    fn exact_match_ties_go_to_the_earlier_mapping() {
        // Two picks with the same source and different inks: the first one wins.
        let mappings = [
            Mapping {
                source: RED,
                ink: BLUE,
                delta_e: 0.0,
            },
            Mapping {
                source: RED,
                ink: Rgb8::new(0, 0, 0),
                delta_e: 0.0,
            },
        ];
        let (out, stats) = run(&[230, 76, 60, 255], &mappings);
        assert_eq!(out, [40, 120, 200, 255]);
        assert_eq!(stats.exact, 1);
        let reversed = [mappings[1], mappings[0]];
        assert_eq!(run(&[230, 76, 60, 255], &reversed).0, [0, 0, 0, 255]);
    }

    #[test]
    fn the_memo_gives_the_same_result_as_per_pixel_lookup() {
        // Large enough to use the memo; colors repeat, include exact sources, transparency, and
        // the exact ΔE tie from gray, so every lookup rule goes through the memo.
        let (w, h) = (400u32, 300u32);
        let mut state = 7u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for i in 0..w * h {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let px = match i % 5 {
                0 => [128, 128, 128, 255],           // ties between the two gray-tie inks
                1 => [230, 76, 60, 255],             // an exact source
                2 => [9, 9, 9, (state >> 24) as u8], // all alphas
                _ => {
                    let v = (state >> 8) & 0x3f3f3f; // few distinct colors: lots of repeats
                    [(v >> 16) as u8 * 4, (v >> 8) as u8 * 4, v as u8 * 4, 255]
                }
            };
            rgba.extend_from_slice(&px);
        }
        let image = ImageRef::new(&rgba, w, h).unwrap();
        let mappings = [
            Mapping {
                source: RED,
                ink: BLUE,
                delta_e: 0.0,
            },
            Mapping {
                source: Rgb8::new(1, 2, 3),
                ink: Rgb8::new(0, 2, 227),
                delta_e: 0.0,
            },
            Mapping {
                source: Rgb8::new(4, 5, 6),
                ink: Rgb8::new(0, 4, 0),
                delta_e: 0.0,
            },
            Mapping {
                source: Rgb8::new(7, 8, 9),
                ink: Rgb8::new(252, 181, 20),
                delta_e: 0.0,
            },
        ];
        assert!((w * h) as usize > MEMO_MIN_PIXELS);
        let inks: Vec<(Mapping, Lab)> = mappings.iter().map(|&m| (m, lab(m.ink))).collect();
        let radii = radius_labs(&mappings);
        let table = Lookup {
            inks: &inks,
            radii: &radii,
        };
        // On (7, 8, 9), transparent pixels are an exact source too.
        for material in [Rgb8::WHITE, Rgb8::new(7, 8, 9)] {
            let mut out = vec![0; rgba.len()];
            let stats = recolor(image, &mappings, material, &mut out).unwrap();
            let mut exact = 0;
            for (pixel, got) in image.pixels().zip(out.as_chunks::<4>().0) {
                let (ink, how) = table.ink(composite(pixel, material));
                assert_eq!(
                    *got,
                    [ink.r, ink.g, ink.b, 255],
                    "pixel {pixel:?} on {material:?}"
                );
                exact += u64::from(how == How::Exact);
            }
            assert_eq!(stats.exact, exact);
            assert_eq!(stats.exact + stats.nearest, u64::from(w * h));
        }
    }

    #[test]
    fn one_mapping_turns_every_pixel_into_its_ink() {
        let mappings = [Mapping {
            source: RED,
            ink: BLUE,
            delta_e: 0.0,
        }];
        let rgba = [
            230, 76, 60, 255, 1, 2, 3, 255, 9, 9, 9, 0, 250, 250, 250, 128,
        ];
        let (out, stats) = run(&rgba, &mappings);
        assert_eq!(out, [40, 120, 200, 255].repeat(4));
        assert_eq!((stats.exact, stats.nearest), (1, 3));
    }

    #[test]
    fn pixels_are_composited_over_the_material_before_matching() {
        let mappings = [
            Mapping {
                source: RED,
                ink: Rgb8::WHITE,
                delta_e: 0.0,
            },
            Mapping {
                source: BLUE,
                ink: BLACK,
                delta_e: 0.0,
            },
        ];
        // Transparent: on white it takes the white ink, on black the black one.
        let transparent = [9, 9, 9, 0];
        assert_eq!(run(&transparent, &mappings).0, [255, 255, 255, 255]);
        let (out, stats) = run_on(&transparent, &mappings, BLACK);
        assert_eq!(out, [0, 0, 0, 255]);
        assert_eq!((stats.exact, stats.nearest), (0, 1));

        // An exact source match is checked after compositing over the material.
        let on_black = [Mapping {
            source: BLACK,
            ink: Rgb8::new(1, 2, 3),
            delta_e: 0.0,
        }];
        let (out, stats) = run_on(&transparent, &on_black, BLACK);
        assert_eq!(out, [1, 2, 3, 255]);
        assert_eq!(stats.exact, 1);
    }

    #[test]
    fn opaque_pixels_do_not_depend_on_the_material() {
        let mappings = [
            Mapping {
                source: RED,
                ink: BLUE,
                delta_e: 0.0,
            },
            Mapping {
                source: Rgb8::new(250, 250, 250),
                ink: BLACK,
                delta_e: 0.0,
            },
        ];
        let rgba = [230, 76, 60, 255, 1, 2, 3, 255, 250, 250, 250, 255];
        let on_white = run(&rgba, &mappings);
        for material in [BLACK, Rgb8::new(128, 128, 128), Rgb8::new(20, 140, 230)] {
            assert_eq!(run_on(&rgba, &mappings, material), on_white, "{material:?}");
        }
    }

    #[test]
    fn no_mappings_passes_composited_pixels_through() {
        let (out, stats) = run(&[10, 20, 30, 0, 10, 20, 30, 255], &[]);
        assert_eq!(out, [255, 255, 255, 255, 10, 20, 30, 255]);
        assert_eq!((stats.exact, stats.nearest), (0, 2));

        // On a material: transparent → the material, translucent → mixed toward it.
        let rgba = [10, 20, 30, 0, 10, 20, 30, 255, 203, 0, 0, 100];
        let (out, _) = run_on(&rgba, &[], BLACK);
        assert_eq!(out, [0, 0, 0, 255, 10, 20, 30, 255, 79, 0, 0, 255]);
    }

    fn run_ranges(
        rgba: &[u8],
        mappings: &[Mapping],
        material: Rgb8,
        ranges: &[MaterialRange],
    ) -> (Vec<u8>, RecolorStats) {
        let image = ImageRef::new(rgba, (rgba.len() / 4) as u32, 1).unwrap();
        let mut out = vec![0; rgba.len()];
        let stats = recolor_with_ranges(image, mappings, material, ranges, &mut out).unwrap();
        (out, stats)
    }

    fn range(pixel: [u8; 4], delta_e: f32) -> MaterialRange {
        MaterialRange {
            pixel: Rgba8::from(pixel),
            delta_e,
        }
    }

    #[test]
    fn ranges_leave_pixels_unprinted_and_transparent() {
        let mappings = [Mapping {
            source: RED,
            ink: BLUE,
            delta_e: 0.0,
        }];
        // Blue, red (an exact source) and a blue one step off.
        let rgba = [40, 120, 200, 255, 230, 76, 60, 255, 41, 120, 200, 255];
        let blue = [range([40, 120, 200, 255], 0.0)];
        let (out, stats) = run_ranges(&rgba, &mappings, Rgb8::WHITE, &blue);
        assert_eq!(out, [0, 0, 0, 0, 40, 120, 200, 255, 40, 120, 200, 255]);
        assert_eq!((stats.unprinted, stats.exact, stats.nearest), (1, 1, 1));
        // A wider ΔE takes the neighbor too.
        let wider = [range([40, 120, 200, 255], 2.0)];
        let (out, stats) = run_ranges(&rgba, &mappings, Rgb8::WHITE, &wider);
        assert_eq!(out[8..], [0, 0, 0, 0]);
        assert_eq!(stats.unprinted, 2);
        // No ranges: exactly `recolor`.
        assert_eq!(
            run_ranges(&rgba, &mappings, Rgb8::WHITE, &[]),
            run(&rgba, &mappings)
        );
    }

    #[test]
    fn ranges_win_over_an_exact_pick() {
        let mappings = [Mapping {
            source: RED,
            ink: BLUE,
            delta_e: 0.0,
        }];
        let red = [range([230, 76, 60, 255], 0.0)];
        let (out, stats) = run_ranges(&[230, 76, 60, 255], &mappings, Rgb8::WHITE, &red);
        assert_eq!(out, [0, 0, 0, 0]);
        assert_eq!((stats.unprinted, stats.exact), (1, 0));
    }

    #[test]
    fn ranges_are_compared_on_the_material() {
        // Transparent (hiding 1,2,3), opaque black, opaque white; no mappings.
        let rgba = [1, 2, 3, 0, 0, 0, 0, 255, 255, 255, 255, 255];
        // The material's own color (what the app adds when a material is chosen): on black, the
        // transparent and the black pixel are both the material, so both are unprinted.
        let (out, _) = run_ranges(&rgba, &[], BLACK, &[range([0, 0, 0, 255], 0.0)]);
        assert_eq!(out, [0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255]);
        // A range made from a transparent pixel follows the material: on white it is white.
        let (out, _) = run_ranges(&rgba, &[], Rgb8::WHITE, &[range([9, 9, 9, 0], 0.0)]);
        assert_eq!(out, [0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn is_unprinted_agrees_with_recolor() {
        let ranges = [range([40, 120, 200, 255], 2.0), range([0, 0, 0, 255], 0.0)];
        let mappings = [Mapping {
            source: RED,
            ink: BLUE,
            delta_e: 0.0,
        }];
        let colors = [
            BLUE,
            Rgb8::new(41, 120, 200),
            Rgb8::new(60, 120, 200),
            RED,
            BLACK,
            Rgb8::WHITE,
        ];
        let (mut unprinted, mut printed) = (0, 0);
        for material in [Rgb8::WHITE, BLACK] {
            for c in colors {
                // An opaque one-pixel image of the color: recolor leaves it transparent exactly
                // when the color is unprinted.
                let (out, _) = run_ranges(&[c.r, c.g, c.b, 255], &mappings, material, &ranges);
                let expected = out[3] == 0;
                assert_eq!(
                    is_unprinted(c, material, &ranges),
                    expected,
                    "{c:?} on {material:?}"
                );
                if expected {
                    unprinted += 1
                } else {
                    printed += 1
                }
            }
        }
        assert!(unprinted > 0 && printed > 0);
        // A range made from a transparent pixel is the material itself.
        assert!(is_unprinted(BLACK, BLACK, &[range([9, 9, 9, 0], 0.0)]));
        assert!(!is_unprinted(
            BLACK,
            Rgb8::WHITE,
            &[range([9, 9, 9, 0], 0.0)]
        ));
    }

    #[test]
    fn the_memo_gives_the_same_result_with_ranges() {
        // Coarse colors (lots of repeats, so the memo is used), every 7th pixel transparent.
        let (w, h) = (400u32, 300u32);
        let rgba: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let v = i.wrapping_mul(2_654_435_761) >> 8;
                let alpha = if i % 7 == 0 { 0 } else { 255 };
                [
                    (v >> 16) as u8 & 0xf0,
                    (v >> 8) as u8 & 0xf0,
                    v as u8 & 0xf0,
                    alpha,
                ]
            })
            .collect();
        let image = ImageRef::new(&rgba, w, h).unwrap();
        let mappings = [
            Mapping {
                source: RED,
                ink: BLUE,
                delta_e: 0.0,
            },
            Mapping {
                source: Rgb8::new(1, 2, 3),
                ink: BLACK,
                delta_e: 0.0,
            },
        ];
        let ranges = [
            range([0, 0, 0, 255], 12.0),
            range([240, 240, 240, 255], 5.0),
        ];
        let material = Rgb8::new(16, 32, 48);
        assert!((w * h) as usize > MEMO_MIN_PIXELS);
        let mut out = vec![0; rgba.len()];
        let stats = recolor_with_ranges(image, &mappings, material, &ranges, &mut out).unwrap();

        let inks: Vec<(Mapping, Lab)> = mappings.iter().map(|&m| (m, lab(m.ink))).collect();
        let radii = radius_labs(&mappings);
        let table = Lookup {
            inks: &inks,
            radii: &radii,
        };
        let labs: Vec<(Lab, f32)> = ranges
            .iter()
            .map(|r| (lab(composite(r.pixel, material)), r.delta_e))
            .collect();
        let mut unprinted = 0;
        for (pixel, got) in image.pixels().zip(out.as_chunks::<4>().0) {
            let color = composite(pixel, material);
            let want = if in_range(&labs, color) {
                unprinted += 1;
                [0, 0, 0, 0]
            } else {
                let (ink, _) = table.ink(color);
                [ink.r, ink.g, ink.b, 255]
            };
            assert_eq!(*got, want, "pixel {pixel:?}");
        }
        assert!(unprinted > 0, "the fixture must hit a range");
        assert_eq!(stats.unprinted, unprinted);
        assert_eq!(
            stats.exact + stats.nearest + stats.unprinted,
            u64::from(w * h)
        );
    }

    // ── Capture radius (a mapping's `delta_e`) ──────────────────────────────────────────────────

    /// A color close to RED (ΔE between 0.5 and 5 from it; checked in the test).
    const NEAR_RED: Rgb8 = Rgb8::new(226, 82, 66);

    fn mapping(source: Rgb8, ink: Rgb8, delta_e: f32) -> Mapping {
        Mapping {
            source,
            ink,
            delta_e,
        }
    }

    #[test]
    fn a_pick_captures_colors_within_its_radius_before_the_nearest_ink_rule() {
        let distance = delta_e_2000_lab(lab(NEAR_RED), lab(RED));
        assert!(distance > 0.5 && distance < 5.0, "fixture: ΔE {distance}");
        // BLUE's ink is nearly NEAR_RED, so without a radius NEAR_RED takes it (nearest ink).
        let near_ink = Rgb8::new(227, 81, 65);
        let pixel = [NEAR_RED.r, NEAR_RED.g, NEAR_RED.b, 255];
        let with_radius = |delta_e| [mapping(RED, BLACK, delta_e), mapping(BLUE, near_ink, 0.0)];
        let (out, stats) = run(&pixel, &with_radius(0.0));
        assert_eq!(out, [227, 81, 65, 255]);
        assert_eq!((stats.captured, stats.nearest), (0, 1));
        // A radius just over the distance: NEAR_RED takes RED's ink.
        let (out, stats) = run(&pixel, &with_radius(distance + 0.1));
        assert_eq!(out, [0, 0, 0, 255]);
        assert_eq!((stats.exact, stats.captured, stats.nearest), (0, 1, 0));
        // Exactly the distance is still inside (`<=`); just under it is outside.
        assert_eq!(run(&pixel, &with_radius(distance)).0, [0, 0, 0, 255]);
        assert_eq!(
            run(&pixel, &with_radius(distance - 0.1)).0,
            [227, 81, 65, 255]
        );
    }

    #[test]
    fn an_exact_match_wins_over_a_radius() {
        // NEAR_RED is inside RED's radius, but it is the second mapping's own source.
        let mappings = [mapping(RED, BLACK, 50.0), mapping(NEAR_RED, BLUE, 0.0)];
        let (out, stats) = run(&[226, 82, 66, 255], &mappings);
        assert_eq!(out, [40, 120, 200, 255]);
        assert_eq!((stats.exact, stats.captured), (1, 0));
    }

    #[test]
    fn overlapping_radii_go_to_the_nearest_source_then_the_earlier_mapping() {
        let toward_blue = Rgb8::new(60, 115, 190);
        let pixel = [toward_blue.r, toward_blue.g, toward_blue.b, 255];
        // Both radii contain the pixel; BLUE's source is nearer.
        let (out, _) = run(
            &pixel,
            &[mapping(RED, BLACK, 100.0), mapping(BLUE, RED, 100.0)],
        );
        assert_eq!(out, [230, 76, 60, 255]);
        // Equal distances (the same source twice): the earlier mapping.
        let (out, _) = run(
            &pixel,
            &[mapping(BLUE, BLACK, 100.0), mapping(BLUE, RED, 100.0)],
        );
        assert_eq!(out, [0, 0, 0, 255]);
    }

    #[test]
    fn ranges_win_over_a_radius() {
        let image = ImageRef::new(&[226, 82, 66, 255], 1, 1).unwrap();
        let ranges = [MaterialRange {
            pixel: Rgba8::new(226, 82, 66, 255),
            delta_e: 1.0,
        }];
        let mut out = [0; 4];
        let stats = recolor_with_ranges(
            image,
            &[mapping(RED, BLACK, 50.0)],
            Rgb8::WHITE,
            &ranges,
            &mut out,
        )
        .unwrap();
        assert_eq!(out, [0, 0, 0, 0]);
        assert_eq!((stats.unprinted, stats.captured), (1, 0));
    }

    #[test]
    fn the_memo_gives_the_same_result_with_radii() {
        // Large enough to use the memo, with repeated colors around two capture radii.
        let (w, h) = (400u32, 300u32);
        let mut state = 11u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let v = (state >> 8) & 0x3f3f3f;
            rgba.extend_from_slice(&[(v >> 16) as u8 * 4, (v >> 8) as u8 * 4, v as u8 * 4, 255]);
        }
        let image = ImageRef::new(&rgba, w, h).unwrap();
        let mappings = [
            mapping(RED, BLACK, 25.0),
            mapping(BLUE, Rgb8::new(250, 250, 0), 15.0),
            mapping(Rgb8::new(128, 128, 128), Rgb8::new(0, 200, 0), 0.0),
        ];
        assert!((w * h) as usize > MEMO_MIN_PIXELS);
        let inks: Vec<(Mapping, Lab)> = mappings.iter().map(|&m| (m, lab(m.ink))).collect();
        let radii = radius_labs(&mappings);
        let table = Lookup {
            inks: &inks,
            radii: &radii,
        };
        let mut out = vec![0; rgba.len()];
        let stats = recolor(image, &mappings, Rgb8::WHITE, &mut out).unwrap();
        let mut counts = RecolorStats::default();
        for (pixel, got) in image.pixels().zip(out.as_chunks::<4>().0) {
            let (ink, how) = table.ink(composite(pixel, Rgb8::WHITE));
            assert_eq!(*got, [ink.r, ink.g, ink.b, 255], "pixel {pixel:?}");
            match how {
                How::Exact => counts.exact += 1,
                How::Captured => counts.captured += 1,
                How::Nearest => counts.nearest += 1,
            }
        }
        assert!(counts.captured > 0, "the fixture must hit a radius");
        assert_eq!(stats, counts);
    }
}
