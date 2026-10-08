use crate::color::{Lab, delta_e_2000_lab, lab};
use crate::{Error, Rgb8};

/// Entries whose name contains this are not real inks (`palettes/pantone.json` has
/// "Pure White (non-palette)" and "Pure Black (non-palette)").
pub const NON_PALETTE_MARKER: &str = "non-palette";

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
/// The order is the file order and decides ties.
#[derive(Debug, Clone)]
pub struct Palette {
    entries: Vec<PaletteEntry>,
    labs: Vec<Lab>,
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

    /// The suggested entry for a color: the nearest one by CIEDE2000, real ink or not (no
    /// preference for real inks over pure white/black). Ties go to the earlier entry.
    pub fn suggest(&self, color: Rgb8) -> PaletteMatch<'_> {
        self.distances(color)
            .reduce(|best, m| if m.delta_e < best.delta_e { m } else { best })
            .expect("Palette::new rejects empty palettes")
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
                delta_e: delta_e_2000_lab(color_lab, entry_lab),
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
    fn suggest_is_the_nearest_entry_even_when_it_is_non_palette() {
        // No preference for real inks: pure white is the nearest entry here, though not 1.5×
        // closer than the gray ink.
        let p = palette(&[
            ("Pure White (non-palette)", [255, 255, 255]),
            ("Gray", [230, 230, 230]),
        ]);
        let color = Rgb8::new(244, 244, 244);
        let (white, gray) = (
            crate::delta_e_2000(color, Rgb8::new(255, 255, 255)),
            crate::delta_e_2000(color, Rgb8::new(230, 230, 230)),
        );
        assert!(
            white < gray && gray < 1.5 * white,
            "precondition: {white} vs {gray}"
        );
        assert_eq!(p.suggest(color).entry.name, "Pure White (non-palette)");
        // Exact colors are suggested exactly.
        assert_eq!(p.suggest(Rgb8::new(230, 230, 230)).entry.name, "Gray");
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
