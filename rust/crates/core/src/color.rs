use palette::color_difference::Ciede2000;
use palette::white_point::D65;
use palette::{FromColor, Srgb};

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

/// CIE L\*a\*b\* with the D65 white point, in `f32` (the `palette` crate's type; R11, D6.1).
pub type Lab = palette::Lab<D65, f32>;

/// Converts an (encoded, not linear) sRGB color to Lab.
pub fn lab(Rgb8 { r, g, b }: Rgb8) -> Lab {
    Lab::from_color(Srgb::new(r, g, b).into_format::<f32>())
}

/// CIEDE2000 color difference between two Lab colors.
///
/// Computed by `palette` with its `libm` backend, i.e. the same float code on every target. For Lab
/// values converted from 8-bit sRGB colors ([`lab`], as [`delta_e_2000`] does) the result is
/// checked to be bit-identical natively and in WASM by the fingerprint in
/// `testdata/baseline/delta-e-fingerprint.tsv`. Other Lab values (e.g. out of the sRGB gamut) are
/// only checked within `1e-4` against the Sharma reference data, on each target separately.
pub fn delta_e_2000_lab(x: Lab, y: Lab) -> f32 {
    x.difference(y)
}

/// CIEDE2000 color difference between two sRGB colors.
pub fn delta_e_2000(x: Rgb8, y: Rgb8) -> f32 {
    delta_e_2000_lab(lab(x), lab(y))
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

    /// Sharma, Wu and Dalal (2005), "The CIEDE2000 color-difference formula: implementation notes,
    /// supplementary test data, and mathematical observations": 34 pairs of Lab colors with their
    /// ΔE to 4 decimals. File: `testdata/ciede2000-sharma.tsv`, the values of
    /// <https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/ciede2000testdata.txt>
    /// unchanged, tab-separated with a `#` header line.
    #[test]
    fn delta_e_matches_the_sharma_reference_data() {
        let data = include_str!("../../../testdata/ciede2000-sharma.tsv");
        let mut failures = Vec::new();
        let mut pairs = 0;
        for line in data.lines().filter(|l| !l.starts_with('#')) {
            let v: Vec<f32> = line.split('\t').map(|x| x.parse().unwrap()).collect();
            let x = Lab::new(v[0], v[1], v[2]);
            let y = Lab::new(v[3], v[4], v[5]);
            let actual = delta_e_2000_lab(x, y);
            // Rounding to 4 decimals is at most 0.5e-4; 1e-4 adds headroom for f32 error.
            if (actual - v[6]).abs() > 1e-4 {
                failures.push(format!("{line}: expected {}, got {actual}", v[6]));
            }
            pairs += 1;
        }
        assert_eq!(pairs, 34);
        assert!(
            failures.is_empty(),
            "{} of 34 pairs differ:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn delta_e_is_zero_for_identical_colors_and_positive_otherwise() {
        let red = Rgb8::new(230, 76, 60);
        assert_eq!(delta_e_2000(red, red), 0.0);
        assert!(delta_e_2000(red, Rgb8::new(40, 120, 200)) > 10.0);
    }
}
