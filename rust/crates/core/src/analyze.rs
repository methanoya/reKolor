use std::collections::HashSet;

use crate::ImageRef;

/// Size and color counts of an image (the existing `image_info`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageStats {
    pub width: u32,
    pub height: u32,
    /// Distinct RGB values, with alpha ignored entirely (existing behavior: no compositing,
    /// transparent pixels counted too; issue I4).
    pub rgb_colors: u64,
    /// Distinct RGBA values.
    pub rgba_colors: u64,
}

pub fn analyze(image: ImageRef<'_>) -> ImageStats {
    let mut rgb = HashSet::new();
    let mut rgba = HashSet::new();
    for p in image.pixels() {
        rgb.insert([p.r, p.g, p.b]);
        rgba.insert(p);
    }
    ImageStats {
        width: image.width(),
        height: image.height(),
        rgb_colors: rgb.len() as u64,
        rgba_colors: rgba.len() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_rgb_ignoring_alpha_and_rgba_separately() {
        let buf = [1, 2, 3, 255, 1, 2, 3, 0, 9, 9, 9, 255, 9, 9, 9, 255];
        let stats = analyze(ImageRef::new(&buf, 2, 2).unwrap());
        assert_eq!(
            stats,
            ImageStats {
                width: 2,
                height: 2,
                rgb_colors: 2,
                rgba_colors: 3
            }
        );
    }
}
