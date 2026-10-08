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
    let mut colors = HashSet::new();
    let mut rgba = HashSet::new();
    for p in image.pixels() {
        colors.insert(composite_over_white(p));
        rgba.insert(p);
    }
    ImageStats {
        width: image.width(),
        height: image.height(),
        colors: colors.len() as u64,
        rgba_colors: rgba.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
