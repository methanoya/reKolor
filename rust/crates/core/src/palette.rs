use deltae::LabValue;

use crate::color::{delta, lab};
use crate::{Error, Rgb8};

/// Entries whose name contains this are not real inks (the 2023 list has
/// "Pure White (non-palette)" and "Pure Black (non-palette)").
pub const NON_PALETTE_MARKER: &str = "non-palette";

/// The 2023 suggestion rule (`palette.ts`): a real ink is suggested unless the nearest
/// non-palette entry is more than this many times closer (issue I5).
pub const NON_PALETTE_BIAS: f32 = 1.5;

/// One named palette color, e.g. `"Pantone 1235"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub name: String,
    pub rgb: Rgb8,
    /// Not a real ink; derived from the name ([`NON_PALETTE_MARKER`]).
    pub non_palette: bool,
}

impl PaletteEntry {
    pub fn new(name: impl Into<String>, rgb: Rgb8) -> Self {
        let name = name.into();
        let non_palette = name.contains(NON_PALETTE_MARKER);
        Self {
            name,
            rgb,
            non_palette,
        }
    }
}

/// A palette entry matched to a color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteMatch<'a> {
    /// Position in the palette (file order); identifies the entry even where RGB values repeat.
    pub index: usize,
    pub entry: &'a PaletteEntry,
    pub delta_e: f32,
}

/// An ordered list of named colors, with Lab values computed once.
/// The order is the file order and decides ties (issue I7).
#[derive(Debug, Clone)]
pub struct Palette {
    entries: Vec<PaletteEntry>,
    labs: Vec<LabValue>,
}

impl Palette {
    pub fn new(entries: Vec<PaletteEntry>) -> Result<Self, Error> {
        if entries.is_empty() {
            return Err(Error::EmptyPalette);
        }
        let labs = entries.iter().map(|e| lab(e.rgb)).collect();
        Ok(Self { entries, labs })
    }

    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    /// The suggestion the 2023 picker made: the nearest real ink, unless the nearest
    /// non-palette entry is more than [`NON_PALETTE_BIAS`] times closer. Ties go to the
    /// earlier entry.
    pub fn suggest(&self, color: Rgb8) -> PaletteMatch<'_> {
        let mut best_ink: Option<PaletteMatch<'_>> = None;
        let mut best_other: Option<PaletteMatch<'_>> = None;
        for m in self.distances(color) {
            let best = if m.entry.non_palette {
                &mut best_other
            } else {
                &mut best_ink
            };
            if best.is_none_or(|b| m.delta_e < b.delta_e) {
                *best = Some(m);
            }
        }
        // 2023: `bestPaletteDistance < bestOutDistance * 1.5 ? bestPaletteColor : bestOutColor`.
        // A missing side counts as infinitely far, as `Number.MAX_VALUE` did.
        let ink_distance = best_ink.map_or(f32::INFINITY, |m| m.delta_e);
        let other_distance = best_other.map_or(f32::INFINITY, |m| m.delta_e);
        let prefer_ink = ink_distance < other_distance * NON_PALETTE_BIAS;
        match (best_ink, best_other) {
            (Some(ink), _) if prefer_ink => ink,
            (_, Some(other)) => other,
            (Some(ink), None) => ink,
            (None, None) => unreachable!("Palette::new rejects empty palettes"),
        }
    }

    /// Up to `k` entries ordered by CIEDE2000 distance; ties go to the earlier entry.
    pub fn nearest(&self, color: Rgb8, k: usize) -> Vec<PaletteMatch<'_>> {
        let mut matches: Vec<_> = self.distances(color).collect();
        matches.sort_by(|a, b| a.delta_e.total_cmp(&b.delta_e).then(a.index.cmp(&b.index)));
        matches.truncate(k);
        matches
    }

    fn distances(&self, color: Rgb8) -> impl Iterator<Item = PaletteMatch<'_>> {
        let color_lab = lab(color);
        self.entries
            .iter()
            .zip(&self.labs)
            .enumerate()
            .map(move |(index, (entry, &entry_lab))| PaletteMatch {
                index,
                entry,
                delta_e: delta(color_lab, entry_lab),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette(entries: &[(&str, [u8; 3])]) -> Palette {
        Palette::new(
            entries
                .iter()
                .map(|&(name, rgb)| PaletteEntry::new(name, rgb.into()))
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn rejects_empty_palette() {
        assert_eq!(Palette::new(vec![]).unwrap_err(), Error::EmptyPalette);
    }

    #[test]
    fn non_palette_is_derived_from_the_name() {
        assert!(PaletteEntry::new("Pure White (non-palette)", Rgb8::default()).non_palette);
        assert!(!PaletteEntry::new("Pantone 100", Rgb8::default()).non_palette);
    }

    #[test]
    fn suggest_prefers_a_real_ink_unless_non_palette_is_much_closer() {
        let p = palette(&[
            ("Pure White (non-palette)", [255, 255, 255]),
            ("Off-white", [240, 240, 235]),
        ]);
        // Pure white is exactly white: infinitely closer.
        assert_eq!(p.suggest(Rgb8::new(255, 255, 255)).index, 0);
        // Exactly the off-white ink: distance 0 beats anything.
        assert_eq!(p.suggest(Rgb8::new(240, 240, 235)).index, 1);
    }

    #[test]
    fn suggest_falls_back_when_one_side_is_missing() {
        let only_inks = palette(&[("A", [0, 0, 0]), ("B", [255, 255, 255])]);
        assert_eq!(only_inks.suggest(Rgb8::new(250, 250, 250)).entry.name, "B");
        let only_other = palette(&[("Pure Black (non-palette)", [0, 0, 0])]);
        assert_eq!(only_other.suggest(Rgb8::new(250, 250, 250)).index, 0);
    }

    #[test]
    fn duplicate_rgb_ties_go_to_the_earlier_entry() {
        // Like Pantone 303 / 547: two names, one RGB.
        let p = palette(&[
            ("far", [250, 250, 250]),
            ("303", [0, 63, 84]),
            ("547", [0, 63, 84]),
        ]);
        assert_eq!(p.suggest(Rgb8::new(0, 60, 80)).entry.name, "303");
        let names: Vec<_> = p
            .nearest(Rgb8::new(0, 60, 80), 3)
            .iter()
            .map(|m| m.entry.name.as_str())
            .collect();
        assert_eq!(names, ["303", "547", "far"]);
        assert_eq!(p.nearest(Rgb8::new(0, 60, 80), 1).len(), 1);
    }
}
