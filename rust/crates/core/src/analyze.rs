use std::collections::HashSet;

use crate::{ImageRef, composite_over_white};

/// Size and color counts of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageStats {
    pub width: u32,
    pub height: u32,
    /// Distinct colors after compositing over white: what [`recolor`](crate::recolor) actually
    /// matches (I4). Fully transparent pixels count as white, whatever RGB they hide.
    pub colors: u64,
    /// Distinct RGBA values, as stored.
    pub rgba_colors: u64,
}

pub fn analyze(image: ImageRef<'_>) -> ImageStats {
    let rgba: HashSet<_> = image.pixels().collect();
    ImageStats {
        width: image.width(),
        height: image.height(),
        colors: color_count(image),
        rgba_colors: rgba.len() as u64,
    }
}

/// Distinct colors after compositing over white ([`ImageStats::colors`]) with fixed memory: one
/// bit per possible color (2^24 bits = 2 MiB), whatever the image. [`analyze`] also counts RGBA
/// values, which needs memory per distinct value; this is the bounded part on its own.
pub fn color_count(image: ImageRef<'_>) -> u64 {
    let mut seen = vec![0u64; 1 << 18];
    let mut count = 0;
    for p in image.pixels() {
        let c = composite_over_white(p);
        let key = (usize::from(c.r) << 16) | (usize::from(c.g) << 8) | usize::from(c.b);
        let (word, bit) = (key >> 6, 1u64 << (key & 63));
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
        let set: HashSet<_> = image.pixels().map(composite_over_white).collect();
        assert_eq!(color_count(image), set.len() as u64);
    }

    #[test]
    fn counts_composited_colors_and_rgba_separately() {
        // Opaque (1,2,3); transparent hiding (7,7,7) → white; opaque white; half-transparent
        // (9,9,9) → (131,131,131). The old count (RGB, alpha ignored) would have been 4.
        let buf = [1, 2, 3, 255, 7, 7, 7, 0, 255, 255, 255, 255, 9, 9, 9, 128];
        let stats = analyze(ImageRef::new(&buf, 2, 2).unwrap());
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
}
