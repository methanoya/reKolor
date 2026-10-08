import type { ColorMismatch, Mapping, PaletteMatch, Rgb, Rgba } from 'rekolor-wasm';

/** One picked color and the ink that replaces it. */
export interface PickEntry {
  id: number;
  /** The stored pixel (straight alpha). */
  pixel: Rgba;
  /** The pixel composited over white: what recolor matches (computed in Rust). */
  matching: Rgb;
  /** The chosen ink (the suggestion unless the user changed it). */
  ink: PaletteMatch;
  /** The nearest inks to choose from, nearest first (WA2: 8). */
  alternatives: PaletteMatch[];
  /** Where it was picked; absent for picks imported from a config. */
  at?: { x: number; y: number };
  /** The color seen on screen differed from the stored pixel (a warning, R10). */
  mismatch?: ColorMismatch;
}

export const css = ({ r, g, b }: Rgb) => `rgb(${r} ${g} ${b})`;
export const cssAlpha = ({ r, g, b, a }: Rgba) => `rgb(${r} ${g} ${b} / ${(a / 255).toFixed(3)})`;
export const hex = ({ r, g, b }: Rgb) =>
  '#' + [r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('');

export const sameRgb = (a: Rgb, b: Rgb) => a.r === b.r && a.g === b.g && a.b === b.b;

export const mappings = (picks: PickEntry[]): Mapping[] =>
  picks.map((p) => ({ source: p.matching, ink: p.ink.rgb }));

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
