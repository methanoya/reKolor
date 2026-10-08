use crate::{Error, ImageRef, Palette, PaletteMatch, Rgb8, Rgba8, composite};

/// Largest per-channel difference between the color the caller saw and the stored pixel that
/// still counts as a match. Above it, [`pick`] returns a [`ColorMismatch`] warning.
///
/// The caller usually reads its color from a scaled, smoothed display canvas, so small
/// differences are expected; large ones point to a coordinate-mapping bug (like the 2023
/// HiDPI picking bug).
pub const PICK_MISMATCH_TOLERANCE: u8 = 8;

/// The result of picking the pixel at an image coordinate (R10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pick<'a> {
    /// The stored pixel.
    pub pixel: Rgba8,
    /// The color used for matching: the pixel composited over the material ([`composite`]).
    pub matching: Rgb8,
    /// The suggested palette entry for `matching` ([`Palette::suggest`]).
    pub suggestion: PaletteMatch<'a>,
    /// Set when the color the caller saw differs from `pixel` by more than
    /// [`PICK_MISMATCH_TOLERANCE`]. A warning only: the pick is still valid.
    pub mismatch: Option<ColorMismatch>,
}

/// A warning that the caller's color doesn't match the stored pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorMismatch {
    pub seen: Rgba8,
    pub stored: Rgba8,
    /// The largest per-channel difference (R, G, B or A).
    pub max_channel_difference: u8,
}

/// Picks the pixel at image coordinates `(x, y)`, composites it over `material` and suggests a
/// palette entry for the result.
///
/// `seen` is the color the caller saw at that position (for debugging); if given, it is compared
/// with the stored pixel and a mismatch is reported as a warning, never as an error.
pub fn pick<'p>(
    image: ImageRef<'_>,
    x: u32,
    y: u32,
    seen: Option<Rgba8>,
    material: Rgb8,
    palette: &'p Palette,
) -> Result<Pick<'p>, Error> {
    let pixel = image.pixel(x, y)?;
    let matching = composite(pixel, material);
    let mismatch = seen.and_then(|seen| {
        let max_channel_difference = [
            seen.r.abs_diff(pixel.r),
            seen.g.abs_diff(pixel.g),
            seen.b.abs_diff(pixel.b),
            seen.a.abs_diff(pixel.a),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        (max_channel_difference > PICK_MISMATCH_TOLERANCE).then_some(ColorMismatch {
            seen,
            stored: pixel,
            max_channel_difference,
        })
    });
    if let Some(m) = &mismatch {
        log::warn!(
            "pick ({x}, {y}): seen {:?} differs from stored {:?} by {}",
            m.seen,
            m.stored,
            m.max_channel_difference
        );
    }
    Ok(Pick {
        pixel,
        matching,
        suggestion: palette.suggest(matching),
        mismatch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaletteEntry;

    fn palette() -> Palette {
        Palette::new(vec![
            PaletteEntry::new("Pure White (non-palette)", Rgb8::new(255, 255, 255)),
            PaletteEntry::new("Pantone 179", Rgb8::new(226, 61, 40)),
        ])
        .unwrap()
    }

    #[test]
    fn picks_the_pixel_composites_it_and_suggests() {
        let buf = [0, 0, 0, 0, 230, 76, 60, 255];
        let image = ImageRef::new(&buf, 2, 1).unwrap();
        let palette = palette();
        let p = pick(image, 1, 0, None, Rgb8::WHITE, &palette).unwrap();
        assert_eq!(p.pixel, Rgba8::new(230, 76, 60, 255));
        assert_eq!(p.matching, Rgb8::new(230, 76, 60));
        assert_eq!(p.suggestion.entry.name, "Pantone 179");
        assert_eq!(p.mismatch, None);

        // A transparent pixel is matched as white.
        let p = pick(image, 0, 0, None, Rgb8::WHITE, &palette).unwrap();
        assert_eq!(p.matching, Rgb8::new(255, 255, 255));
        assert_eq!(p.suggestion.index, 0);
    }

    #[test]
    fn matches_over_the_material() {
        let buf = [9, 9, 9, 0, 230, 76, 60, 255, 203, 0, 0, 100];
        let image = ImageRef::new(&buf, 3, 1).unwrap();
        let palette = Palette::new(vec![
            PaletteEntry::new("Pure White (non-palette)", Rgb8::new(255, 255, 255)),
            PaletteEntry::new("Pure Black (non-palette)", Rgb8::new(0, 0, 0)),
            PaletteEntry::new("Pantone 179", Rgb8::new(226, 61, 40)),
        ])
        .unwrap();
        let black = Rgb8::new(0, 0, 0);

        // Transparent: the material, whatever RGB it hides.
        let p = pick(image, 0, 0, None, black, &palette).unwrap();
        assert_eq!(p.pixel, Rgba8::new(9, 9, 9, 0));
        assert_eq!(p.matching, black);
        assert_eq!(p.suggestion.entry.name, "Pure Black (non-palette)");
        // Opaque: unchanged.
        let p = pick(image, 1, 0, None, black, &palette).unwrap();
        assert_eq!(p.matching, Rgb8::new(230, 76, 60));
        // Translucent: mixed toward the material.
        let p = pick(image, 2, 0, None, black, &palette).unwrap();
        assert_eq!(p.matching, Rgb8::new(79, 0, 0));
    }

    #[test]
    fn out_of_bounds_is_an_error() {
        let buf = [0; 4];
        let image = ImageRef::new(&buf, 1, 1).unwrap();
        assert!(matches!(
            pick(image, 1, 0, None, Rgb8::WHITE, &palette()),
            Err(Error::OutOfBounds { .. })
        ));
    }

    #[test]
    fn mismatch_is_a_warning_above_the_tolerance_only() {
        let buf = [100, 100, 100, 255];
        let image = ImageRef::new(&buf, 1, 1).unwrap();
        let palette = palette();
        let within = Rgba8::new(100 + PICK_MISMATCH_TOLERANCE, 100, 100, 255);
        assert_eq!(
            pick(image, 0, 0, Some(within), Rgb8::WHITE, &palette)
                .unwrap()
                .mismatch,
            None
        );
        let beyond = Rgba8::new(100, 100, 100 - PICK_MISMATCH_TOLERANCE - 1, 255);
        let p = pick(image, 0, 0, Some(beyond), Rgb8::WHITE, &palette).unwrap();
        assert_eq!(
            p.mismatch,
            Some(ColorMismatch {
                seen: beyond,
                stored: Rgba8::new(100, 100, 100, 255),
                max_channel_difference: PICK_MISMATCH_TOLERANCE + 1,
            })
        );
        // Still a valid pick.
        assert_eq!(p.pixel, Rgba8::new(100, 100, 100, 255));
    }
}
