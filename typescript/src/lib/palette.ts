// Reading an ink palette: the built-in `palettes/pantone.json`, which the engine loads at startup, or
// a palette the user uploads. The format is described by `palettes/palette.schema.json`; this reader
// accepts exactly what the schema does (a test checks that they agree), within the app's limits.
import type { PaletteData } from 'rekolor-wasm';
import { LIMITS } from './limits';

/**
 * Converts a palette (`{ "<name>": { "rgb": [r, g, b] }, … }`, as in `palettes/pantone.json`) to
 * the WASM package's `PaletteData`. `Object.entries` keeps file order, which decides ties. Throws
 * an `Error` saying what is wrong: not an object, no inks, more than `LIMITS.paletteInks`, an
 * empty name, or an `rgb` that isn't three whole numbers 0–255. Other keys of an entry are ignored.
 */
export function parsePaletteJson(text: string): PaletteData {
  // `unknown` forces a check before use (unlike `any`, which would allow anything unchecked).
  const json: unknown = JSON.parse(text);
  if (typeof json !== 'object' || json === null || Array.isArray(json)) {
    throw new Error('palette JSON must be an object of { rgb: [r, g, b] } entries');
  }
  const entries = Object.entries(json);
  if (entries.length === 0) throw new Error('the palette needs at least one ink');
  if (entries.length > LIMITS.paletteInks) {
    throw new Error(
      `the palette has ${entries.length} inks; at most ${LIMITS.paletteInks} are allowed`,
    );
  }
  return {
    entries: entries.map(([name, value]) => {
      if (name === '') throw new Error('every ink needs a name');
      // `as` tells the compiler what shape to assume; the checks right below make that safe.
      const rgb =
        typeof value === 'object' && value !== null && !Array.isArray(value)
          ? (value as { rgb?: unknown }).rgb
          : undefined;
      if (
        !Array.isArray(rgb) ||
        rgb.length !== 3 ||
        !rgb.every((c) => Number.isInteger(c) && c >= 0 && c <= 255)
      ) {
        throw new Error(`palette entry ${JSON.stringify(name)}: rgb must be three integers 0–255`);
      }
      const [r, g, b] = rgb as [number, number, number];
      return { name, rgb: { r, g, b } };
    }),
  };
}
