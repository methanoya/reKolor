use rekolor_core as core;
use tsify::Ts;
use wasm_bindgen::prelude::*;

use crate::outcome::{
    NearestOutcome, SuggestOutcome, error_object, ok_object, outcome_js, whole_u32,
};
use crate::{ErrorInfo, Outcome, PaletteData, PaletteMatch, PaletteMatches, Rgb};

#[wasm_bindgen]
extern "C" {
    /// `Outcome<Palette>`: built by hand because a class instance can't go through serde.
    #[wasm_bindgen(typescript_type = "Outcome<Palette>")]
    pub type PaletteOutcome;
}

/// A palette kept on the WASM side, with Lab values computed once.
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

    /// The suggested entry for a color: the nearest one by CIEDE2000 (non-palette entries
    /// included); ties go to the earlier entry.
    pub fn suggest(&self, color: Ts<Rgb>) -> SuggestOutcome {
        let outcome = color
            .to_rust()
            .map_err(ErrorInfo::from)
            .map(|c| PaletteMatch::from(self.inner.suggest(c.into())));
        outcome_js(Outcome::from(outcome))
    }

    /// Up to `k` entries ordered by distance; ties go to the earlier entry. `k` must be a whole
    /// number from 0 to 2^32 − 1.
    pub fn nearest(&self, color: Ts<Rgb>, k: f64) -> NearestOutcome {
        let outcome = whole_u32("k", k).and_then(|k| {
            let c = color.to_rust().map_err(ErrorInfo::from)?;
            Ok(PaletteMatches {
                matches: self
                    .inner
                    .nearest(c.into(), k as usize)
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            })
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
