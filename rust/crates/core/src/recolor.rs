use crate::color::{Lab, delta_e_2000_lab, lab};
use crate::{Error, ImageRef, Rgb8, composite_over_white};

/// One picked color and the ink that replaces it (`from` → `to` in the 2023 UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub source: Rgb8,
    pub ink: Rgb8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecolorStats {
    /// Pixels whose composited color equals a mapping's source exactly.
    pub exact: u64,
    /// All other pixels.
    pub nearest: u64,
}

/// Recolors `image` into `out`, which must have the same length as the source buffer.
///
/// Contract (behavior decisions I1, I2, I8, I9: all kept from the existing implementation):
/// 1. each pixel is composited over white ([`composite_over_white`]); the output is opaque;
/// 2. if the result equals a mapping's `source`, it takes that mapping's ink (first match wins);
/// 3. otherwise it takes the **ink** nearest to it by CIEDE2000 (on a tie, the earlier mapping);
/// 4. **no mappings**: the output is the composited copy (the image as it looks on white);
/// 5. **one mapping**: no special case, so every pixel takes that ink.
///
/// ΔE is bit-identical on native and WASM builds ([`delta_e_2000_lab`](crate::delta_e_2000_lab)),
/// so ties resolve the same way everywhere.
pub fn recolor(
    image: ImageRef<'_>,
    mappings: &[Mapping],
    out: &mut [u8],
) -> Result<RecolorStats, Error> {
    let input = image.as_bytes();
    if out.len() != input.len() {
        return Err(Error::OutputLength {
            expected: input.len(),
            actual: out.len(),
        });
    }

    let inks: Vec<(Mapping, Lab)> = mappings.iter().map(|&m| (m, lab(m.ink))).collect();
    let pixel_count = input.len() / 4;
    let mut memo =
        (pixel_count > MEMO_MIN_PIXELS && !inks.is_empty() && inks.len() < MEMO_MAX_INKS)
            .then(|| vec![0u16; 1 << 24]);
    let mut stats = RecolorStats::default();
    for (pixel, dst) in image.pixels().zip(out.as_chunks_mut::<4>().0) {
        let color = composite_over_white(pixel);
        let (ink, exact) = match memo.as_deref_mut() {
            Some(memo) => memoized(memo, &inks, color),
            None => lookup(&inks, color),
        };
        *dst = [ink.r, ink.g, ink.b, 255];
        if exact {
            stats.exact += 1;
        } else {
            stats.nearest += 1;
        }
    }
    log::debug!(
        "recolor: {}×{}, {} mappings, exact {}, nearest {}",
        image.width(),
        image.height(),
        mappings.len(),
        stats.exact,
        stats.nearest
    );
    Ok(stats)
}

/// Images above this many pixels remember each color's answer (below it, the 32 MiB table costs
/// more than it saves).
const MEMO_MIN_PIXELS: usize = 1 << 16;
/// The memo stores `mapping index + 1` in 15 bits.
const MEMO_MAX_INKS: usize = 0x7fff;
const MEMO_EXACT: u16 = 0x8000;

/// [`lookup`] remembered per composited color (one `u16` for each of the 2^24 colors: 0 = not yet
/// computed, else the mapping index + 1, with [`MEMO_EXACT`] for exact matches). The answer depends
/// only on the color, so results are identical; images repeat colors, so most pixels skip the
/// CIEDE2000 work. Memory is fixed (32 MiB) whatever the image.
fn memoized(memo: &mut [u16], inks: &[(Mapping, Lab)], color: Rgb8) -> (Rgb8, bool) {
    let key = (usize::from(color.r) << 16) | (usize::from(color.g) << 8) | usize::from(color.b);
    let entry = memo[key];
    if entry != 0 {
        return (
            inks[usize::from(entry & !MEMO_EXACT) - 1].0.ink,
            entry & MEMO_EXACT != 0,
        );
    }
    let (index, exact) = lookup_index(inks, color);
    // `inks` is non-empty when the memo is used, so `lookup_index` always finds an index.
    let index = index.expect("memo is only used with mappings");
    memo[key] = (index as u16 + 1) | if exact { MEMO_EXACT } else { 0 };
    (inks[index].0.ink, exact)
}

/// The ink for one composited pixel color, and whether it was an exact source match.
fn lookup(inks: &[(Mapping, Lab)], color: Rgb8) -> (Rgb8, bool) {
    match lookup_index(inks, color) {
        (Some(index), exact) => (inks[index].0.ink, exact),
        // No mappings: the composited pixel passes through.
        (None, _) => (color, false),
    }
}

/// Which mapping a composited color takes: an exact source match first (first match wins), else
/// the nearest ink (ties: the earlier mapping). `None` only when there are no mappings.
fn lookup_index(inks: &[(Mapping, Lab)], color: Rgb8) -> (Option<usize>, bool) {
    if let Some(index) = inks.iter().position(|(m, _)| m.source == color) {
        return (Some(index), true);
    }
    // The pixel's Lab value is computed once instead of once per ink (same value each time).
    let color_lab = lab(color);
    let mut best: Option<(usize, f32)> = None;
    for (index, &(_, ink_lab)) in inks.iter().enumerate() {
        let distance = delta_e_2000_lab(color_lab, ink_lab);
        if best.is_none_or(|(_, d)| d > distance) {
            best = Some((index, distance));
        }
    }
    (best.map(|(index, _)| index), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb8 = Rgb8::new(230, 76, 60);
    const BLUE: Rgb8 = Rgb8::new(40, 120, 200);

    fn run(rgba: &[u8], mappings: &[Mapping]) -> (Vec<u8>, RecolorStats) {
        let image = ImageRef::new(rgba, (rgba.len() / 4) as u32, 1).unwrap();
        let mut out = vec![0; rgba.len()];
        let stats = recolor(image, mappings, &mut out).unwrap();
        (out, stats)
    }

    #[test]
    fn rejects_output_of_wrong_length() {
        let image = ImageRef::new(&[0; 8], 2, 1).unwrap();
        assert_eq!(
            recolor(image, &[], &mut [0; 4]).unwrap_err(),
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
            },
            Mapping {
                source: BLUE,
                ink: Rgb8::new(231, 77, 61),
            },
        ];
        let (out, stats) = run(&[231, 76, 60, 255], &mappings);
        assert_eq!(out, [231, 77, 61, 255]);
        assert_eq!((stats.exact, stats.nearest), (0, 1));
    }

    #[test]
    fn nearest_ink_ties_go_to_the_earlier_mapping() {
        // Two different inks at exactly the same CIEDE2000 distance from gray, 39.605984 (R11: the
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
            },
            Mapping {
                source: BLUE,
                ink: b,
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
            },
            Mapping {
                source: RED,
                ink: Rgb8::new(0, 0, 0),
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
            },
            Mapping {
                source: Rgb8::new(1, 2, 3),
                ink: Rgb8::new(0, 2, 227),
            },
            Mapping {
                source: Rgb8::new(4, 5, 6),
                ink: Rgb8::new(0, 4, 0),
            },
            Mapping {
                source: Rgb8::new(7, 8, 9),
                ink: Rgb8::new(252, 181, 20),
            },
        ];
        assert!((w * h) as usize > MEMO_MIN_PIXELS);
        let mut out = vec![0; rgba.len()];
        let stats = recolor(image, &mappings, &mut out).unwrap();

        let inks: Vec<(Mapping, Lab)> = mappings.iter().map(|&m| (m, lab(m.ink))).collect();
        let mut exact = 0;
        for (pixel, got) in image.pixels().zip(out.as_chunks::<4>().0) {
            let (ink, is_exact) = lookup(&inks, composite_over_white(pixel));
            assert_eq!(*got, [ink.r, ink.g, ink.b, 255], "pixel {pixel:?}");
            exact += u64::from(is_exact);
        }
        assert_eq!(stats.exact, exact);
        assert_eq!(stats.exact + stats.nearest, u64::from(w * h));
    }

    #[test]
    fn one_mapping_turns_every_pixel_into_its_ink() {
        let mappings = [Mapping {
            source: RED,
            ink: BLUE,
        }];
        let rgba = [
            230, 76, 60, 255, 1, 2, 3, 255, 9, 9, 9, 0, 250, 250, 250, 128,
        ];
        let (out, stats) = run(&rgba, &mappings);
        assert_eq!(out, [40, 120, 200, 255].repeat(4));
        assert_eq!((stats.exact, stats.nearest), (1, 3));
    }

    #[test]
    fn no_mappings_passes_composited_pixels_through() {
        let (out, stats) = run(&[10, 20, 30, 0, 10, 20, 30, 255], &[]);
        assert_eq!(out, [255, 255, 255, 255, 10, 20, 30, 255]);
        assert_eq!((stats.exact, stats.nearest), (0, 2));
    }
}
