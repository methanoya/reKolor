use rekolor_core as core;
use tsify::Ts;
use wasm_bindgen::prelude::*;

use crate::outcome::{NearestOutcome, SuggestOutcome, error_object, ok_object, outcome_js};
use crate::{ErrorInfo, Outcome, PaletteData, PaletteMatch, PaletteMatches, Rgb};

#[wasm_bindgen]
extern "C" {
    /// `Outcome<Palette>`: built by hand because a class instance can't go through serde.
    #[wasm_bindgen(typescript_type = "Outcome<Palette>")]
    pub type PaletteOutcome;
}

/// A palette kept on the WASM side, with Lab values computed once (T1).
#[wasm_bindgen]
#[derive(Debug)]
pub struct Palette {
    inner: core::Palette,
}

#[wasm_bindgen]
impl Palette {
    /// Creates a palette from ordered entries (e.g. `palettes/pantone.json` converted with
    /// `Object.entries`, which keeps file order).
    pub fn create(data: Ts<PaletteData>) -> PaletteOutcome {
        let outcome = data
            .to_rust()
            .map_err(ErrorInfo::from)
            .and_then(Palette::from_data);
        match outcome {
            Ok(palette) => ok_object(palette.into()),
            Err(error) => error_object(&error),
        }
        .unchecked_into()
    }

    /// The number of entries.
    #[wasm_bindgen(getter)]
    pub fn length(&self) -> u32 {
        self.inner.entries().len() as u32
    }

    /// The suggestion for a color, with the existing rule (nearest real ink, unless a
    /// non-palette entry is more than 1.5× closer).
    pub fn suggest(&self, color: Ts<Rgb>) -> SuggestOutcome {
        let outcome = color
            .to_rust()
            .map_err(ErrorInfo::from)
            .map(|c| PaletteMatch::from(self.inner.suggest(c.into())));
        outcome_js(Outcome::from(outcome))
    }

    /// Up to `k` entries ordered by distance; ties go to the earlier entry.
    pub fn nearest(&self, color: Ts<Rgb>, k: u32) -> NearestOutcome {
        let outcome = color
            .to_rust()
            .map_err(ErrorInfo::from)
            .map(|c| PaletteMatches {
                matches: self
                    .inner
                    .nearest(c.into(), k as usize)
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            });
        outcome_js(Outcome::from(outcome))
    }
}

impl Palette {
    /// Rust-side constructor (not exported); used by `create` and by tests.
    pub fn from_data(data: PaletteData) -> Result<Palette, ErrorInfo> {
        let entries = data
            .entries
            .into_iter()
            .map(|e| core::PaletteEntry::new(e.name, e.rgb.into()))
            .collect();
        Ok(Palette {
            inner: core::Palette::new(entries)?,
        })
    }

    pub(crate) fn core(&self) -> &core::Palette {
        &self.inner
    }
}
