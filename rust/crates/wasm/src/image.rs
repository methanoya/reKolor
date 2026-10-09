// The image the user opened, as a JavaScript class (`SourceImage`) backed by memory inside the
// WASM module. JavaScript holds a handle to it; the pixels themselves stay on the Rust side.
use rekolor_core as core;
use tsify::Ts;
use wasm_bindgen::prelude::*;

use crate::outcome::{
    ColorCountOutcome, ImageStatsOutcome, PickOutcome, RecolorOutcome, RgbOutcome,
    UnprintedColorsOutcome, error_object, ok_object, outcome_js, whole_u32,
};
use crate::{
    ColorCount, ErrorInfo, ImageStats, MaterialRange, Outcome, Palette, RecolorRequest, Rgb, Rgba,
    UnprintedCheck, UnprintedColors,
};

#[wasm_bindgen]
extern "C" {
    /// `Outcome<SourceImage>`: built by hand because a class instance can't go through serde.
    #[wasm_bindgen(typescript_type = "Outcome<SourceImage>")]
    pub type SourceImageOutcome;
}

// `#[wasm_bindgen]` on a struct exports it as a JavaScript class. Its fields stay private; JS
// reaches the data only through the methods below. The object lives in WASM memory until JS calls
// its generated `free()` method (or it is garbage-collected).
/// The source image kept on the WASM side: its pixels are copied in once and every later
/// call works on them, so picks always read the real source pixel.
#[wasm_bindgen]
#[derive(Debug)]
pub struct SourceImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

// Methods JavaScript can call: `image.recolor(...)`, `image.pick(...)`, and so on.
#[wasm_bindgen]
impl SourceImage {
    // A static factory (`SourceImage.create(...)` in JS) rather than a constructor, because a
    // constructor can't return an error value. `Vec<u8>` takes a copy of the JS `Uint8Array`. The
    // sizes arrive as `f64` (a JS number) and are checked by `whole_u32`, since a plain `u32`
    // parameter would silently accept `1.5` or `-1`.
    /// Copies an RGBA8 buffer (e.g. `imageData.data`) of `width × height × 4` bytes.
    /// `width` and `height` must be whole numbers from 0 to 2^32 − 1.
    pub fn create(rgba: Vec<u8>, width: f64, height: f64) -> SourceImageOutcome {
        let created = whole_u32("width", width)
            .and_then(|w| Ok((w, whole_u32("height", height)?)))
            .and_then(|(w, h)| SourceImage::from_rgba(rgba, w, h));
        match created {
            Ok(image) => ok_object(image.into()),
            Err(error) => error_object(&error),
        }
        .unchecked_into()
    }

    // `getter` exposes the method as a read-only property: `image.width` in JS, not
    // `image.width()`.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    // `Ts<Rgb>` is the JS object as received; `to_rust()` converts it and fails if its shape is
    // wrong (missing field, out-of-range number), which becomes an `InvalidInput` outcome.
    /// Size and color counts, `colors` on the given material. Counting RGBA values needs memory
    /// per distinct value (hundreds of MB for a large noisy image); `colorCount` is the bounded
    /// alternative.
    pub fn analyze(&self, material: Ts<Rgb>) -> ImageStatsOutcome {
        let outcome = material
            .to_rust()
            .map_err(ErrorInfo::from)
            .map(|m| ImageStats::from(core::analyze(self.view(), m.into())));
        outcome_js(Outcome::from(outcome))
    }

    // `js_name` sets the JavaScript name (camelCase) for the Rust method `color_count`.
    /// Distinct colors after compositing over the material (`ImageStats.colors`), with fixed
    /// memory (2 MiB) whatever the image.
    #[wasm_bindgen(js_name = colorCount)]
    pub fn color_count(&self, material: Ts<Rgb>) -> ColorCountOutcome {
        let outcome = material
            .to_rust()
            .map_err(ErrorInfo::from)
            .map(|m| ColorCount {
                // At most 2^24, exact as a JS number.
                colors: core::color_count(self.view(), m.into()) as f64,
            });
        outcome_js(Outcome::from(outcome))
    }

    // `out: &mut [u8]` is a temporary copy of the caller's `Uint8Array` in WASM memory:
    // wasm-bindgen copies the array in, the function writes into the copy, and wasm-bindgen copies
    // the result back into the caller's array when the call returns.
    /// Recolors the image into `out`, which must be `width × height × 4` bytes. To write
    /// straight into an `ImageData`, pass `new Uint8Array(imageData.data.buffer)`.
    ///
    /// Each pixel is composited over `request.material` and takes the nearest ink (an exact source
    /// match takes its own ink; ties go to the earlier mapping), fully opaque. A pixel within a
    /// mapping's `deltaE` (its capture radius) of its source takes that mapping's ink instead (the
    /// nearest such source wins; ties: the earlier mapping). With **no mappings**
    /// the output is the composited copy (the image as it looks on the material); with **one
    /// mapping** every pixel takes that ink. ΔE is bit-identical to a native build, so ties
    /// resolve the same way. A pixel within a `materialRanges` entry (checked first) takes no ink
    /// and is **transparent** (`0, 0, 0, 0`), so the material shows through.
    /// At most 256 mappings (the palette-config limit), since the work grows with each one; the
    /// same for ranges. Every `deltaE` (mapping or range) is from 0 to 100 (`invalidInput`).
    pub fn recolor(&self, request: Ts<RecolorRequest>, out: &mut [u8]) -> RecolorOutcome {
        let outcome = request
            .to_rust()
            .map_err(ErrorInfo::from)
            .and_then(|request| {
                if request.mappings.len() > rekolor_config::MAX_PICKS {
                    return Err(ErrorInfo {
                        kind: crate::ErrorKind::TooManyMappings,
                        message: format!(
                            "{} mappings; at most {} are allowed",
                            request.mappings.len(),
                            rekolor_config::MAX_PICKS
                        ),
                    });
                }
                let ranges = core_ranges(&request.material_ranges)?;
                if let Some(m) = request
                    .mappings
                    .iter()
                    .find(|m| !(0.0..=100.0).contains(&m.delta_e))
                {
                    return Err(ErrorInfo {
                        kind: crate::ErrorKind::InvalidInput,
                        message: format!(
                            "a mapping's deltaE must be from 0 to 100, got {}",
                            m.delta_e
                        ),
                    });
                }
                let mappings: Vec<core::Mapping> =
                    request.mappings.into_iter().map(Into::into).collect();
                Ok(crate::RecolorStats::from(core::recolor_with_ranges(
                    self.view(),
                    &mappings,
                    request.material.into(),
                    &ranges,
                    out,
                )?))
            });
        outcome_js(Outcome::from(outcome))
    }

    /// Picks the pixel at image coordinates `(x, y)` (whole numbers), composites it over
    /// `material` and suggests a palette entry for the result. `seen` is the color the caller saw
    /// there (for debugging); a difference beyond the tolerance comes back as `mismatch`, a warning
    /// only.
    pub fn pick(
        &self,
        x: f64,
        y: f64,
        seen: Option<Ts<Rgba>>,
        material: Ts<Rgb>,
        palette: &Palette,
    ) -> PickOutcome {
        let outcome = whole_u32("x", x)
            .and_then(|x| Ok((x, whole_u32("y", y)?)))
            .and_then(|(x, y)| {
                // `seen` is optional: `transpose` turns `Option<Result<..>>` into
                // `Result<Option<..>>`, so `?` can report a malformed color while `None` stays
                // `None`.
                let seen = seen
                    .map(|s| s.to_rust())
                    .transpose()
                    .map_err(ErrorInfo::from)?;
                let material = material.to_rust().map_err(ErrorInfo::from)?;
                let pick = core::pick(
                    self.view(),
                    x,
                    y,
                    seen.map(Into::into),
                    material.into(),
                    palette.core(),
                )?;
                Ok(crate::Pick::from(pick))
            });
        outcome_js(Outcome::from(outcome))
    }
}

// A free function rather than a method: JavaScript calls it as `composite(pixel, material)`.
/// A pixel composited over the material: the color recolor matches (for picks without a position,
/// e.g. imported ones, when the material changes).
#[wasm_bindgen]
pub fn composite(pixel: Ts<Rgba>, material: Ts<Rgb>) -> RgbOutcome {
    let outcome = pixel.to_rust().map_err(ErrorInfo::from).and_then(|p| {
        let m = material.to_rust().map_err(ErrorInfo::from)?;
        Ok(Rgb::from(core::composite(p.into(), m.into())))
    });
    outcome_js(Outcome::from(outcome))
}

// Rust-only methods: this second `impl` block has no `#[wasm_bindgen]`, so JavaScript never sees
// it.
/// Which of `colors` (as recolor matches them, composited over the material) recolor leaves
/// unprinted with these material ranges: the same test it applies to every pixel. At most 256
/// colors and 256 ranges, each `deltaE` from 0 to 100.
#[wasm_bindgen(js_name = unprintedColors)]
pub fn unprinted_colors(request: Ts<UnprintedCheck>) -> UnprintedColorsOutcome {
    let outcome = request
        .to_rust()
        .map_err(ErrorInfo::from)
        .and_then(|request| {
            if request.colors.len() > rekolor_config::MAX_PICKS {
                return Err(ErrorInfo {
                    kind: crate::ErrorKind::TooManyMappings,
                    message: format!(
                        "{} colors; at most {} are allowed",
                        request.colors.len(),
                        rekolor_config::MAX_PICKS
                    ),
                });
            }
            let ranges = core_ranges(&request.material_ranges)?;
            let material = request.material.into();
            Ok(UnprintedColors {
                unprinted: request
                    .colors
                    .into_iter()
                    .map(|c| core::is_unprinted(c.into(), material, &ranges))
                    .collect(),
            })
        });
    outcome_js(Outcome::from(outcome))
}

/// Checks the material ranges of a request and converts them to the core type: at most 256
/// (`tooManyRanges`), each `deltaE` from 0 to 100 (`invalidInput`); the first invalid one ends
/// the call.
fn core_ranges(ranges: &[MaterialRange]) -> Result<Vec<core::MaterialRange>, ErrorInfo> {
    if ranges.len() > rekolor_config::MAX_UNPRINTED {
        return Err(ErrorInfo {
            kind: crate::ErrorKind::TooManyRanges,
            message: format!(
                "{} material ranges; at most {} are allowed",
                ranges.len(),
                rekolor_config::MAX_UNPRINTED
            ),
        });
    }
    ranges
        .iter()
        .map(|r| {
            if (0.0..=100.0).contains(&r.delta_e) {
                Ok(core::MaterialRange {
                    pixel: r.pixel.into(),
                    delta_e: r.delta_e,
                })
            } else {
                Err(ErrorInfo {
                    kind: crate::ErrorKind::InvalidInput,
                    message: format!("deltaE must be from 0 to 100, got {}", r.delta_e),
                })
            }
        })
        .collect()
}

impl SourceImage {
    /// Rust-side constructor (not exported); used by `create` and by tests.
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<SourceImage, ErrorInfo> {
        core::ImageRef::new(&rgba, width, height)?;
        Ok(SourceImage {
            width,
            height,
            rgba,
        })
    }

    fn view(&self) -> core::ImageRef<'_> {
        core::ImageRef::new(&self.rgba, self.width, self.height).expect("validated in from_rgba")
    }
}
