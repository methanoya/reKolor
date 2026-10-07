use deltae::{DE2000, Delta, LabValue};
use lab::Lab;

/// An sRGB color, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb8 {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl From<[u8; 3]> for Rgb8 {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self { r, g, b }
    }
}

/// An sRGB color with straight (not premultiplied) alpha, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl From<[u8; 4]> for Rgba8 {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Self { r, g, b, a }
    }
}

/// Composites a pixel over white with the existing integer formula `(255 - a) + a * c / 255`,
/// which truncates. Can't overflow: the result is at most `255 - a + a = 255`.
pub fn composite_over_white(Rgba8 { r, g, b, a }: Rgba8) -> Rgb8 {
    let channel = |c: u8| (255 - a) + (u16::from(a) * u16::from(c) / 255) as u8;
    Rgb8::new(channel(r), channel(g), channel(b))
}

/// CIEDE2000 color difference between two sRGB colors (`lab` + `deltae`, as before; R11).
pub fn delta_e_2000(x: Rgb8, y: Rgb8) -> f32 {
    delta(lab(x), lab(y))
}

pub(crate) fn lab(Rgb8 { r, g, b }: Rgb8) -> LabValue {
    let Lab { l, a, b } = Lab::from_rgb(&[r, g, b]);
    LabValue { l, a, b }
}

pub(crate) fn delta(x: LabValue, y: LabValue) -> f32 {
    *x.delta(y, DE2000).value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_keeps_opaque_and_whitens_transparent() {
        assert_eq!(
            composite_over_white(Rgba8::new(10, 20, 30, 255)),
            Rgb8::new(10, 20, 30)
        );
        assert_eq!(
            composite_over_white(Rgba8::new(10, 20, 30, 0)),
            Rgb8::new(255, 255, 255)
        );
    }

    #[test]
    fn composite_truncates() {
        // (255 - 100) + 100 * 203 / 255 = 155 + 79.6… → 234 (the 2023 TypeScript rounded to 235)
        assert_eq!(composite_over_white(Rgba8::new(203, 0, 0, 100)).r, 234);
        // (255 - 200) + 200 * 10 / 255 = 55 + 7.8… → 62
        assert_eq!(composite_over_white(Rgba8::new(10, 0, 0, 200)).r, 62);
    }

    #[test]
    fn composite_never_overflows() {
        for a in 0..=255u8 {
            for c in 0..=255u8 {
                composite_over_white(Rgba8::new(c, c, c, a));
            }
        }
    }

    #[test]
    fn delta_e_is_zero_for_identical_colors_and_positive_otherwise() {
        let red = Rgb8::new(230, 76, 60);
        assert_eq!(delta_e_2000(red, red), 0.0);
        assert!(delta_e_2000(red, Rgb8::new(40, 120, 200)) > 10.0);
    }
}
