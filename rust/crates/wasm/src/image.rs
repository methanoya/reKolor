use rekolor_core as core;
use tsify::Ts;
use wasm_bindgen::prelude::*;

use crate::outcome::{PickOutcome, RecolorOutcome, error_object, ok_object, outcome_js, to_ts};
use crate::{ErrorInfo, ImageStats, Outcome, Palette, RecolorRequest, Rgba};

#[wasm_bindgen]
extern "C" {
    /// `Outcome<SourceImage>`: built by hand because a class instance can't go through serde.
    #[wasm_bindgen(typescript_type = "Outcome<SourceImage>")]
    pub type SourceImageOutcome;
}

/// The source image kept on the WASM side (T1): its pixels are copied in once and every later
/// call works on them, so picks always read the real source pixel (R10).
#[wasm_bindgen]
#[derive(Debug)]
pub struct SourceImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[wasm_bindgen]
impl SourceImage {
    /// Copies an RGBA8 buffer (e.g. `imageData.data`) of `width × height × 4` bytes.
    pub fn create(rgba: Vec<u8>, width: u32, height: u32) -> SourceImageOutcome {
        match SourceImage::from_rgba(rgba, width, height) {
            Ok(image) => ok_object(image.into()),
            Err(error) => error_object(&error),
        }
        .unchecked_into()
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Size and color counts.
    pub fn analyze(&self) -> Ts<ImageStats> {
        to_ts(&core::analyze(self.view()).into())
    }

    /// Recolors the image into `out`, which must be `width × height × 4` bytes. To write
    /// straight into an `ImageData`, pass `new Uint8Array(imageData.data.buffer)`.
    pub fn recolor(&self, request: Ts<RecolorRequest>, out: &mut [u8]) -> RecolorOutcome {
        let outcome = request
            .to_rust()
            .map_err(ErrorInfo::from)
            .and_then(|request| {
                let mappings: Vec<core::Mapping> =
                    request.mappings.into_iter().map(Into::into).collect();
                Ok(crate::RecolorStats::from(core::recolor(
                    self.view(),
                    &mappings,
                    out,
                )?))
            });
        outcome_js(Outcome::from(outcome))
    }

    /// Picks the pixel at image coordinates `(x, y)` and suggests a palette entry for it.
    /// `seen` is the color the caller saw there (for debugging); a difference beyond the
    /// tolerance comes back as `mismatch`, a warning only.
    pub fn pick(&self, x: u32, y: u32, seen: Option<Ts<Rgba>>, palette: &Palette) -> PickOutcome {
        let outcome = seen
            .map(|s| s.to_rust())
            .transpose()
            .map_err(ErrorInfo::from)
            .and_then(|seen| {
                let pick = core::pick(self.view(), x, y, seen.map(Into::into), palette.core())?;
                Ok(crate::Pick::from(pick))
            });
        outcome_js(Outcome::from(outcome))
    }
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
