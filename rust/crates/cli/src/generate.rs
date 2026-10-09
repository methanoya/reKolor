//! First-version palette configs: an image's predominant colors that are most different from
//! each other, each mapped to its suggested ink, up to N distinct inks.
//!
//! Rule (deterministic):
//! 1. Group pixels by their matching color (composited over the material, as an interactive pick
//!    is), coarsened to 16 levels per channel so photo noise doesn't fragment a color into
//!    thousands of groups. Each group's representative is its most frequent exact pixel value.
//! 2. Candidates: the [`CANDIDATES`] most frequent groups ("predominant").
//! 3. Greedy farthest-point selection: start with the most frequent candidate, then repeatedly
//!    take the candidate whose CIEDE2000 distance to the nearest already-chosen color is
//!    largest (ties: more frequent first).
//! 4. A candidate whose suggested ink is already taken is skipped, so all inks are distinct.
//!
//! The selection doesn't depend on N, so smaller palettes are prefixes of larger ones.

// `HashMap` is a key → value dictionary.
use std::collections::HashMap;

use rekolor_core::{ImageRef, Palette, Rgb8, Rgba8, composite, delta_e_2000};

/// How many of the most frequent color groups are considered.
pub const CANDIDATES: usize = 64;

/// One generated pick: a pixel value from the image and the palette entry it maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedPick {
    pub rgba: Rgba8,
    pub ink_index: usize,
}

/// Up to `max` picks with distinct inks, in selection order; take the first N for size N. Colors
/// are matched as composited over `material`.
pub fn picks(
    image: ImageRef<'_>,
    palette: &Palette,
    material: Rgb8,
    max: usize,
) -> Vec<GeneratedPick> {
    // Exact pixel counts. `entry(p).or_default()` gets the count for this pixel value, inserting 0
    // the first time; `*... += 1` adds one to the stored count.
    let mut exact: HashMap<Rgba8, u64> = HashMap::new();
    for p in image.pixels() {
        *exact.entry(p).or_default() += 1;
    }
    // Sorted so that everything below is independent of hash-map iteration order.
    let mut exact: Vec<(Rgba8, u64)> = exact.into_iter().collect();
    exact.sort_unstable_by_key(|&(p, _)| (p.r, p.g, p.b, p.a));

    // Groups: total count and most frequent exact pixel (ties: the smaller value, from the sort).
    // Each group is keyed by the color's top 4 bits per channel (`>> 4` divides by 16); its value
    // holds (total pixels, most frequent exact pixel, that pixel's count). `group.0`, `.1`, `.2`
    // are the tuple's fields by position.
    let mut groups: HashMap<[u8; 3], (u64, Rgba8, u64)> = HashMap::new();
    for (pixel, count) in exact {
        let Rgb8 { r, g, b } = composite(pixel, material);
        let group = groups
            .entry([r >> 4, g >> 4, b >> 4])
            .or_insert((0, pixel, 0));
        group.0 += count;
        if count > group.2 {
            group.1 = pixel;
            group.2 = count;
        }
    }
    // Most frequent groups first (`b.1.cmp(&a.1)` sorts descending); the key breaks ties so the
    // order never depends on the hash map.
    let mut candidates: Vec<([u8; 3], u64, Rgba8)> = groups
        .into_iter()
        .map(|(key, (total, pixel, _))| (key, total, pixel))
        .collect();
    candidates.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    candidates.truncate(CANDIDATES);

    // Each candidate as (stored pixel, matching color, pixel count).
    let mut remaining: Vec<(Rgba8, Rgb8, u64)> = candidates
        .into_iter()
        .map(|(_, count, pixel)| (pixel, composite(pixel, material), count))
        .collect();
    let mut chosen: Vec<(GeneratedPick, Rgb8)> = Vec::new();
    while chosen.len() < max && !remaining.is_empty() {
        // Farthest from everything chosen so far (the first pick: the most frequent). `max_by` with
        // a custom comparison: the larger "distance to the nearest chosen color" wins. Before
        // anything is chosen, every distance is infinity, so the count decides: the most frequent
        // color goes first.
        let best = remaining
            .iter()
            .enumerate()
            .max_by(|(ia, a), (ib, b)| {
                let distance = |c: Rgb8| {
                    chosen
                        .iter()
                        .map(|(_, m)| delta_e_2000(c, *m))
                        .fold(f32::INFINITY, f32::min)
                };
                distance(a.1)
                    .total_cmp(&distance(b.1))
                    .then(a.2.cmp(&b.2))
                    .then(ib.cmp(ia)) // equal: prefer the earlier (more frequent) candidate
            })
            .map(|(i, _)| i)
            .expect("remaining is not empty");
        // Take the winner out of the candidates; `_` ignores the count.
        let (pixel, matching, _) = remaining.remove(best);
        let ink_index = palette.suggest(matching).index;
        if chosen.iter().any(|(p, _)| p.ink_index == ink_index) {
            continue; // ink already taken: skip this candidate for good
        }
        chosen.push((
            GeneratedPick {
                rgba: pixel,
                ink_index,
            },
            matching,
        ));
    }
    chosen.into_iter().map(|(pick, _)| pick).collect()
}

// Unit tests with small hand-made images and a five-color palette.
#[cfg(test)]
mod tests {
    use super::*;
    use rekolor_core::PaletteEntry;

    fn palette() -> Palette {
        Palette::new(vec![
            PaletteEntry::new("White", Rgb8::new(255, 255, 255)),
            PaletteEntry::new("Black", Rgb8::new(0, 0, 0)),
            PaletteEntry::new("Red", Rgb8::new(220, 30, 30)),
            PaletteEntry::new("Blue", Rgb8::new(30, 60, 220)),
            PaletteEntry::new("Yellow", Rgb8::new(250, 220, 20)),
        ])
        .unwrap()
    }

    /// An image with the given (color, pixel count) blocks.
    fn image(blocks: &[([u8; 4], usize)]) -> Vec<u8> {
        blocks
            .iter()
            .flat_map(|&(c, n)| std::iter::repeat_n(c, n).flatten())
            .collect()
    }

    fn view(rgba: &[u8]) -> ImageRef<'_> {
        ImageRef::new(rgba, (rgba.len() / 4) as u32, 1).unwrap()
    }

    #[test]
    fn starts_with_the_most_frequent_color_then_goes_far() {
        let rgba = image(&[
            ([250, 250, 250, 255], 50), // most frequent: white-ish
            ([245, 245, 245, 255], 30), // close to white: same group/ink
            ([10, 10, 10, 255], 10),    // far from white
            ([215, 35, 35, 255], 5),    // red
        ]);
        let p = palette();
        let picks = picks(view(&rgba), &p, Rgb8::WHITE, 3);
        let inks: Vec<_> = picks
            .iter()
            .map(|g| p.entries()[g.ink_index].name.as_str())
            .collect();
        assert_eq!(inks, ["White", "Black", "Red"]);
        assert_eq!(picks[0].rgba, Rgba8::new(250, 250, 250, 255));
    }

    #[test]
    fn inks_are_distinct_and_the_count_is_up_to_n() {
        // Many different reds and whites, but only two inks match them.
        let rgba = image(&[
            ([250, 250, 250, 255], 10),
            ([200, 40, 40, 255], 10),
            ([230, 20, 20, 255], 10),
            ([180, 30, 30, 255], 10),
        ]);
        let p = palette();
        let picks = picks(view(&rgba), &p, Rgb8::WHITE, 16);
        let mut inks: Vec<_> = picks.iter().map(|g| g.ink_index).collect();
        assert_eq!(inks.len(), 2);
        inks.dedup();
        assert_eq!(inks.len(), 2);
    }

    #[test]
    fn smaller_palettes_are_prefixes_and_the_result_is_deterministic() {
        let rgba: Vec<u8> = (0..400u32)
            .flat_map(|i| [(i * 7) as u8, (i * 13) as u8, (i * 29) as u8, 255])
            .collect();
        let p = palette();
        let five = picks(view(&rgba), &p, Rgb8::WHITE, 5);
        let three = picks(view(&rgba), &p, Rgb8::WHITE, 3);
        assert_eq!(&five[..3], &three[..]);
        assert_eq!(picks(view(&rgba), &p, Rgb8::WHITE, 5), five);
    }

    #[test]
    fn transparent_pixels_count_as_the_material() {
        let rgba = image(&[([0, 0, 0, 0], 50), ([10, 10, 10, 255], 10)]);
        let p = palette();
        let on_white = picks(view(&rgba), &p, Rgb8::WHITE, 2);
        assert_eq!(p.entries()[on_white[0].ink_index].name, "White");
        assert_eq!(on_white[0].rgba, Rgba8::new(0, 0, 0, 0));
        assert_eq!(p.entries()[on_white[1].ink_index].name, "Black");

        // On black, the transparent pixels and the near-black ones are one group (the transparent
        // pixels, more frequent, represent it), so only one ink is found.
        let on_black = picks(view(&rgba), &p, Rgb8::new(0, 0, 0), 2);
        assert_eq!(on_black.len(), 1);
        assert_eq!(p.entries()[on_black[0].ink_index].name, "Black");
        assert_eq!(on_black[0].rgba, Rgba8::new(0, 0, 0, 0));
    }
}
