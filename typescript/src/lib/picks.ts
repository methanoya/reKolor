// The data behind the pick list (picked colors and their inks) and the unprinted colors, plus
// small color helpers. The color types (`Rgb`, `Rgba`, …) are generated from the Rust structs, so
// both sides always agree on their shape.
import type { ColorMismatch, Mapping, PaletteMatch, Rgb, Rgba } from 'rekolor-wasm';

// `field?: Type` is an optional field: it may be missing.
/** One picked color and the ink that replaces it. */
export interface PickEntry {
  id: number;
  /** The stored pixel (straight alpha). */
  pixel: Rgba;
  /** The pixel composited over the material: what recolor matches (computed in Rust). */
  matching: Rgb;
  /** The chosen ink (the suggestion unless the user changed it). */
  ink: PaletteMatch;
  /** The nearest inks to choose from, nearest first (8). */
  alternatives: PaletteMatch[];
  /** Where it was picked; absent for picks imported from a config. */
  at?: { x: number; y: number };
  /** The color seen on screen differed from the stored pixel (a warning). */
  mismatch?: ColorMismatch;
}

// `Object.freeze` makes the shared color objects read-only at runtime, so no code can change
// them by accident.
/** The default material: compositing over it is the behavior from before the material color. */
export const WHITE: Rgb = Object.freeze({ r: 255, g: 255, b: 255 });
export const BLACK: Rgb = Object.freeze({ r: 0, g: 0, b: 0 });

// Colors as CSS text: `rgb(r g b)`, with alpha `rgb(r g b / 0.5)`, or `#rrggbb` hex.
// `({ r, g, b }: Rgb) =>` unpacks the object's fields in the parameter list.
export const css = ({ r, g, b }: Rgb) => `rgb(${r} ${g} ${b})`;
export const cssAlpha = ({ r, g, b, a }: Rgba) => `rgb(${r} ${g} ${b} / ${(a / 255).toFixed(3)})`;
export const hex = ({ r, g, b }: Rgb) =>
  '#' + [r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('');

/** `#rrggbb` (as an `<input type="color">` gives it) to a color. */
export const fromHex = (text: string): Rgb => {
  // Parse the six hex digits as one number, then take each channel's 8 bits (`>>` shifts right,
  // `& 255` keeps the lowest 8 bits).
  const n = Number.parseInt(text.slice(1, 7), 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
};

export const sameRgb = (a: Rgb, b: Rgb) => a.r === b.r && a.g === b.g && a.b === b.b;

// The engine's view of the picks: each picked color (as composited) → its ink's color.
export const mappings = (picks: PickEntry[]): Mapping[] =>
  picks.map((p) => ({ source: p.matching, ink: p.ink.rgb }));

/** A color left unprinted: pixels within `deltaE` of it take no ink; the material shows. */
export interface RangeEntry {
  id: number;
  /** The clicked pixel, as stored (composited over the material in Rust on each recolor). */
  pixel: Rgba;
  deltaE: number;
  /** The slider's maximum: 40, or 100 for an entry that came from a file above 40. */
  maxDeltaE: number;
  /** Where it was clicked (a square marker on the original); absent for the material's range. */
  at?: { x: number; y: number };
  /** The material's own color: added when a material is chosen, and follows it. */
  material?: boolean;
}

/** A Shift-drag of a pick's marker to the source pixel (x, y); `done` on release. */
export interface PickMove {
  /** Increments per drag, so a finished drag's last update is not mixed with the next drag's. */
  drag: number;
  id: number;
  x: number;
  y: number;
  /** The color shown there (for the mismatch warning), as for a click. */
  seen: Rgba | undefined;
  done: boolean;
}
