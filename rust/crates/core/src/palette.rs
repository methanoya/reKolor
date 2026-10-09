// The ink palette (e.g. the Pantone list in `palettes/pantone.json`) and the search for the inks
// nearest to a color.
use crate::color::{Lab, delta_e_2000_lab, lab};
use crate::{Error, Rgb8};

// `&str` is a borrowed piece of text; a string literal like this one lives for the whole program.
/// Entries whose name contains this are not real inks (`palettes/pantone.json` has
/// "Pure White (non-palette)" and "Pure Black (non-palette)").
pub const NON_PALETTE_MARKER: &str = "non-palette";

// `String` is owned, growable text (unlike `&str`, which only borrows text).
/// One named palette color, e.g. `"Pantone 1235"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub name: String,
    pub rgb: Rgb8,
    /// Not a real ink; derived from the name ([`NON_PALETTE_MARKER`]).
    pub non_palette: bool,
}

impl PaletteEntry {
    // `impl Into<String>` accepts anything convertible to a `String` (a `&str` or a `String`), so
    // callers can pass `"Pantone 1235"` directly; `.into()` does the conversion.
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

// The `<'a>` lifetime says the match borrows its entry from a `Palette`: a match can't outlive
// the palette it came from, and no entry is copied.
/// A palette entry matched to a color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteMatch<'a> {
    /// Position in the palette (file order); identifies the entry even where RGB values repeat.
    pub index: usize,
    pub entry: &'a PaletteEntry,
    pub delta_e: f32,
}

// `Vec<T>` is Rust's growable list (an array on the heap). The Lab value of each entry is computed
// once here, because every color comparison needs it and the palette has hundreds of entries.
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
        // `.iter().map(...).collect()` walks the entries, converts each, and gathers the results
        // into a new `Vec` (the type comes from the `labs` field it is stored in).
        let labs = entries.iter().map(|e| lab(e.rgb)).collect();
        Ok(Self { entries, labs })
    }

    // Read-only access to the entries; returning `&[...]` (a slice) lets callers read but not
    // change them.
    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    /// The suggested entry for a color: the nearest one by CIEDE2000, real ink or not (no
    /// preference for real inks over pure white/black). Ties go to the earlier entry.
    pub fn suggest(&self, color: Rgb8) -> PaletteMatch<'_> {
        self.distances(color)
            // Keep the nearest so far; on an exact tie (`<`, not `<=`) the earlier entry stays, so
            // file order decides ties.
            .reduce(|best, m| if m.delta_e < best.delta_e { m } else { best })
            // `reduce` returns `None` for an empty list; `expect` would panic with this message,
            // but `new` never builds an empty palette, so it can't happen.
            .expect("Palette::new rejects empty palettes")
    }

    /// Up to `k` entries ordered by CIEDE2000 distance; ties go to the earlier entry.
    pub fn nearest(&self, color: Rgb8, k: usize) -> Vec<PaletteMatch<'_>> {
        let mut matches: Vec<_> = self.distances(color).collect();
        // Sort by ΔE, then by position for equal ΔE. `total_cmp` orders floats completely (it also
        // defines where NaN goes), which a plain `<` can't.
        matches.sort_by(|a, b| a.delta_e.total_cmp(&b.delta_e).then(a.index.cmp(&b.index)));
        matches.truncate(k);
        matches
    }

    // Every entry with its ΔE to `color`, in file order. `'_` lets the compiler infer the lifetime
    // (the returned matches borrow from `self`). Not `pub`: a helper for the two methods above.
    fn distances(&self, color: Rgb8) -> impl Iterator<Item = PaletteMatch<'_>> {
        let color_lab = lab(color);
        self.entries
            .iter()
            .zip(&self.labs)
            .enumerate()
            // `zip` pairs each entry with its Lab value, `enumerate` adds the position; `move`
            // makes the closure take its own copy of `color_lab`, since the iterator outlives this
            // function call.
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
