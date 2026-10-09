// Image statistics shown in the app's toolbar and printed by `rekolor analyze`: size and how many
// distinct colors the image has. `HashSet` is a set: it keeps each distinct value once.
use std::collections::HashSet;

use crate::{ImageRef, Rgb8, composite};

/// Size and color counts of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageStats {
    pub width: u32,
    pub height: u32,
    /// Distinct colors after compositing over the material: what [`recolor`](crate::recolor)
    /// actually matches. Fully transparent pixels count as the material, whatever RGB they
    /// hide.
    pub colors: u64,
    /// Distinct RGBA values, as stored.
    pub rgba_colors: u64,
}

/// Size and color counts; `colors` depends on the material, `rgba_colors` doesn't.
pub fn analyze(image: ImageRef<'_>, material: Rgb8) -> ImageStats {
    // Collect every pixel into a set; its size is the number of distinct RGBA values. `HashSet<_>`
    // lets the compiler infer the element type (`Rgba8`).
    let rgba: HashSet<_> = image.pixels().collect();
    ImageStats {
        width: image.width(),
        height: image.height(),
        colors: color_count(image, material),
        rgba_colors: rgba.len() as u64,
    }
}

/// Distinct colors after compositing over the material ([`ImageStats::colors`]) with fixed
/// memory: one bit per possible color (2^24 bits = 2 MiB), whatever the image. [`analyze`] also
/// counts RGBA values, which needs memory per distinct value; this is the bounded part on its own.
pub fn color_count(image: ImageRef<'_>, material: Rgb8) -> u64 {
    // A bitmap with one bit per possible RGB color: 2^24 colors / 64 bits per `u64` = 2^18 words.
    // `let mut` declares a variable that can change (Rust variables are read-only by default).
    let mut seen = vec![0u64; 1 << 18];
    let mut count = 0;
    for p in image.pixels() {
        let c = composite(p, material);
        // The color as one number 0..2^24 (red in the high bits), then which word and which bit
        // hold it: `key >> 6` divides by 64, `key & 63` is the remainder.
        let key = (usize::from(c.r) << 16) | (usize::from(c.g) << 8) | usize::from(c.b);
        let (word, bit) = (key >> 6, 1u64 << (key & 63));
        // First time this color is seen: set its bit and count it.
        if seen[word] & bit == 0 {
            seen[word] |= bit;
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_count_matches_a_set_count_on_every_fixture_kind() {
        // Includes all 256 alpha values and colors at both ends of every channel.
        let mut buf = Vec::new();
        for a in 0..=255u8 {
            for c in [0u8, 1, 127, 128, 254, 255] {
                buf.extend_from_slice(&[c, 255 - c, a, a]);
            }
        }
        let image = ImageRef::new(&buf, 6, 256).unwrap();
        for material in [Rgb8::WHITE, Rgb8::new(0, 0, 0), Rgb8::new(20, 140, 230)] {
            let set: HashSet<_> = image.pixels().map(|p| composite(p, material)).collect();
            assert_eq!(
                color_count(image, material),
                set.len() as u64,
                "{material:?}"
            );
        }
    }

    #[test]
    fn counts_composited_colors_and_rgba_separately() {
        // Opaque (1,2,3); transparent hiding (7,7,7) → white; opaque white; half-transparent
        // (9,9,9) → (131,131,131). Counting RGB with alpha ignored would give 4.
        let buf = [1, 2, 3, 255, 7, 7, 7, 0, 255, 255, 255, 255, 9, 9, 9, 128];
        let stats = analyze(ImageRef::new(&buf, 2, 2).unwrap(), Rgb8::WHITE);
        assert_eq!(
            stats,
            ImageStats {
                width: 2,
                height: 2,
                colors: 3,
                rgba_colors: 4
            }
        );
    }

    #[test]
    fn colors_are_counted_on_the_material() {
        // Transparent (7,7,7,0) and opaque black: one color on black, two on white.
        let buf = [7, 7, 7, 0, 0, 0, 0, 255];
        let image = ImageRef::new(&buf, 2, 1).unwrap();
        assert_eq!(analyze(image, Rgb8::new(0, 0, 0)).colors, 1);
        assert_eq!(analyze(image, Rgb8::WHITE).colors, 2);
        assert_eq!(analyze(image, Rgb8::new(0, 0, 0)).rgba_colors, 2);
    }
}
