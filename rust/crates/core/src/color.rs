// Color types and color math. `use` brings names from other crates or modules into scope; here
// from the `palette` crate (color science), not to be confused with this crate's own `palette.rs`
// (the list of inks).
use palette::color_difference::Ciede2000;
use palette::white_point::D65;
use palette::{FromColor, Srgb};

// sRGB is the standard color space of screens and image files; each channel is 0–255 (`u8`).
/// An sRGB color, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb8 {
    // An associated constant, used as `Rgb8::WHITE`.
    /// The default material: compositing over it is the behavior before the material color.
    pub const WHITE: Self = Self::new(255, 255, 255);

    // `const fn`: can also run at compile time, so it can build constants like `WHITE` above.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

// Implementing the standard `From` trait (a trait is like an interface) lets callers convert a
// 3-byte array with `Rgb8::from([r, g, b])` or `[r, g, b].into()`. The parameter `[r, g, b]`
// unpacks the array into three variables right in the signature ("destructuring").
impl From<[u8; 3]> for Rgb8 {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self { r, g, b }
    }
}

// The same with an alpha (opacity) channel: 0 is fully transparent, 255 fully opaque.
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

// The parameter `Rgba8 { r, g, b, a }` destructures the pixel into its four channels.
/// Composites a pixel over the material color (the garment or substrate), per channel
/// `(a·c + (255 − a)·m) / 255`, truncating. Mixes the stored (encoded) sRGB values, not linear
/// light. `a = 255` gives the pixel's color and `a = 0` the material, whatever RGB a transparent
/// pixel hides.
///
/// Over [`Rgb8::WHITE`] this is exactly the existing formula `(255 − a) + a·c / 255`, since
/// `255·(255 − a)` divides by 255, so white reproduces the behavior from before the material.
pub fn composite(Rgba8 { r, g, b, a }: Rgba8, material: Rgb8) -> Rgb8 {
    // Widen to 32 bits first: `a * c` can reach 255 × 255, which doesn't fit in a `u8`.
    let a = u32::from(a);
    // The sum is at most a·255 + (255 − a)·255 = 255·255 (the terms share `a`), so the quotient
    // fits in a u8.
    // `|c, m| ...` is a closure (an inline function) that captures `a` from the line above;
    // `as u8` converts back to a byte (it would truncate, but the result always fits, see above).
    let channel = |c: u8, m: u8| ((a * u32::from(c) + (255 - a) * u32::from(m)) / 255) as u8;
    Rgb8::new(
        channel(r, material.r),
        channel(g, material.g),
        channel(b, material.b),
    )
}

// CIE Lab describes colors the way people perceive them: L is lightness, a is green–red, b is
// blue–yellow. Distances in Lab match perceived differences far better than distances in RGB, which
// is why the engine converts colors to Lab before comparing them. D65 is the standard daylight
// white the conversion is relative to. `type` gives an existing type a shorter name.
/// CIE L\*a\*b\* with the D65 white point, in `f32` (the `palette` crate's type).
pub type Lab = palette::Lab<D65, f32>;

// `into_format::<f32>()` scales the 0–255 bytes to 0.0–1.0 floats, which `palette` expects.
/// Converts an (encoded, not linear) sRGB color to Lab.
pub fn lab(Rgb8 { r, g, b }: Rgb8) -> Lab {
    Lab::from_color(Srgb::new(r, g, b).into_format::<f32>())
}

// CIEDE2000 is the standard formula for "how different do these two colors look" (ΔE). About 1
// is the smallest difference most people notice; the engine always picks the ink with the smallest
// ΔE to a pixel.
/// CIEDE2000 color difference between two Lab colors.
///
/// Computed by `palette` with its `libm` backend, i.e. the same float code on every target. For Lab
/// values converted from 8-bit sRGB colors ([`lab`], as [`delta_e_2000`] does) the result is
/// checked to be bit-identical natively and in WASM by the fingerprint in
/// `testdata/baseline/delta-e-fingerprint.tsv`. Other Lab values (e.g. out of the sRGB gamut) are
/// only checked within `1e-4` against the Sharma reference data, on each target separately.
pub fn delta_e_2000_lab(x: Lab, y: Lab) -> f32 {
    // `difference` comes from the `Ciede2000` trait imported at the top: importing a trait makes
    // its methods available on the types that implement it.
    x.difference(y)
}

/// CIEDE2000 color difference between two sRGB colors.
pub fn delta_e_2000(x: Rgb8, y: Rgb8) -> f32 {
    delta_e_2000_lab(lab(x), lab(y))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Rgb8 = Rgb8::new(0, 0, 0);

    #[test]
    fn composite_keeps_opaque_and_turns_transparent_into_the_material() {
        for material in [Rgb8::WHITE, BLACK, Rgb8::new(20, 140, 230)] {
            assert_eq!(
                composite(Rgba8::new(10, 20, 30, 255), material),
                Rgb8::new(10, 20, 30)
            );
            assert_eq!(composite(Rgba8::new(10, 20, 30, 0), material), material);
        }
    }

    #[test]
    fn composite_truncates() {
        // (255 - 100) + 100 * 203 / 255 = 155 + 79.6… → 234 (not rounded to 235)
        assert_eq!(composite(Rgba8::new(203, 0, 0, 100), Rgb8::WHITE).r, 234);
        // (255 - 200) + 200 * 10 / 255 = 55 + 7.8… → 62
        assert_eq!(composite(Rgba8::new(10, 0, 0, 200), Rgb8::WHITE).r, 62);
    }

    #[test]
    fn composite_mixes_toward_the_material() {
        // (100·203 + 155·m) / 255 for red,
        // 155·m / 255 for green and blue.
        let pixel = Rgba8::new(203, 0, 0, 100);
        assert_eq!(composite(pixel, Rgb8::WHITE), Rgb8::new(234, 155, 155));
        assert_eq!(
            composite(pixel, Rgb8::new(128, 128, 128)),
            Rgb8::new(157, 77, 77)
        );
        assert_eq!(composite(pixel, BLACK), Rgb8::new(79, 0, 0));
    }

    #[test]
    fn composite_over_white_is_the_existing_formula() {
        // The formula from before the material, kept here as the reference.
        let existing = |c: u8, a: u8| (255 - a) + (u16::from(a) * u16::from(c) / 255) as u8;
        for a in 0..=255u8 {
            for c in 0..=255u8 {
                let got = composite(Rgba8::new(c, 255 - c, c / 2, a), Rgb8::WHITE);
                let want = Rgb8::new(existing(c, a), existing(255 - c, a), existing(c / 2, a));
                assert_eq!(got, want, "c = {c}, a = {a}");
            }
        }
    }

    #[test]
    fn composite_stays_between_the_pixel_and_the_material() {
        // Every alpha, pixel channel and material channel: the result never leaves the range the
        // two ends span (so the `as u8` can't wrap), and the ends are exact.
        for a in 0..=255u8 {
            for c in 0..=255u8 {
                for m in 0..=255u8 {
                    let got = composite(Rgba8::new(c, c, c, a), Rgb8::new(m, m, m)).r;
                    assert!(
                        c.min(m) <= got && got <= c.max(m),
                        "c = {c}, m = {m}, a = {a}"
                    );
                    if a == 255 {
                        assert_eq!(got, c);
                    } else if a == 0 {
                        assert_eq!(got, m);
                    }
                }
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
