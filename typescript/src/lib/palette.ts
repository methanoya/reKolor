import type { PaletteData } from 'rekolor-wasm';

/**
 * Converts `palettes/pantone.json` (`{ "<name>": { "rgb": [r, g, b] }, … }`) to the WASM package's
 * `PaletteData`. `Object.entries` keeps file order, which decides ties.
 */
export function parsePaletteJson(text: string): PaletteData {
  const json: unknown = JSON.parse(text);
  if (typeof json !== 'object' || json === null || Array.isArray(json)) {
    throw new Error('palette JSON must be an object of { rgb: [r, g, b] } entries');
  }
  return {
    entries: Object.entries(json).map(([name, value]) => {
      const rgb = (value as { rgb?: unknown }).rgb;
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
