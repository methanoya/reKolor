// X3 (c): the rekolor-wasm contract as an app sees it, at runtime.
// Type-level guarantees are in `types.check.ts` (checked by `tsc`, not run).

import { readFile } from 'node:fs/promises';
import { beforeAll, describe, expect, test } from 'vitest';
import {
  Palette,
  SourceImage,
  type PaletteData,
  type RecolorRequest,
  type Rgb,
} from 'rekolor-wasm';
import { loadWasm, unwrap } from './wasm';

const RED: Rgb = { r: 230, g: 76, b: 60 };
const PANTONE_285: Rgb = { r: 58, g: 117, b: 196 };

/** `palettes/pantone.json` converted the way the app will do it: `Object.entries` keeps file order. */
async function pantoneData(): Promise<PaletteData> {
  const url = new URL('../../../../../palettes/pantone.json', import.meta.url);
  const json = JSON.parse(await readFile(url, 'utf8')) as Record<
    string,
    { rgb: [number, number, number] }
  >;
  return {
    entries: Object.entries(json).map(([name, { rgb: [r, g, b] }]) => ({ name, rgb: { r, g, b } })),
  };
}

/** A 2×1 image: opaque red, then fully transparent. */
function redAndTransparent(): SourceImage {
  return unwrap(SourceImage.create(new Uint8Array([230, 76, 60, 255, 0, 0, 0, 0]), 2, 1));
}

const redToBlue: RecolorRequest = { mappings: [{ source: RED, ink: PANTONE_285 }] };

beforeAll(loadWasm);

describe('Palette', () => {
  test('is created from palettes/pantone.json and keeps file order', async () => {
    const palette = unwrap(Palette.create(await pantoneData()));
    expect(palette.length).toBe(909);

    // The 2023 JS suggested Pantone 1235 for this color (see testdata/baseline).
    const yellow = unwrap(palette.suggest({ r: 255, g: 184, b: 0 }));
    expect(yellow).toMatchObject({ name: 'Pantone 1235', nonPalette: false });
    expect(yellow.rgb).toEqual({ r: 252, g: 181, b: 20 });

    const white = unwrap(palette.suggest({ r: 255, g: 255, b: 255 }));
    expect(white).toMatchObject({ name: 'Pure White (non-palette)', nonPalette: true, deltaE: 0 });

    // Pantone 303 and 547 share an RGB value; file order decides, so 303 comes first.
    const twins = unwrap(palette.nearest({ r: 0, g: 63, b: 84 }, 2)).matches;
    expect(twins.map((m) => m.name)).toEqual(['Pantone 303', 'Pantone 547']);
    expect(twins[0]!.index).toBeLessThan(twins[1]!.index);
    // I7: the suggestion follows the same rule.
    expect(unwrap(palette.suggest({ r: 0, g: 63, b: 84 })).name).toBe('Pantone 303');
    palette.free();
  });

  test('invalid palette data comes back as an error value', () => {
    const empty = Palette.create({ entries: [] });
    expect(empty.status === 'error' && empty.error.kind).toBe('emptyPalette');

    const malformed = Palette.create({ entries: 'nope' } as unknown as PaletteData);
    expect(malformed.status === 'error' && malformed.error.kind).toBe('invalidInput');
  });
});

describe('SourceImage', () => {
  test('create returns a typed outcome', () => {
    const image = redAndTransparent();
    expect([image.width, image.height]).toEqual([2, 1]);
    // Pixel 2 is transparent, so it counts as white.
    expect(image.analyze()).toEqual({ width: 2, height: 1, colors: 2, rgbaColors: 2 });

    const wrongLength = SourceImage.create(new Uint8Array(7), 2, 1);
    expect(wrongLength.status).toBe('error');
    if (wrongLength.status === 'error') {
      expect(wrongLength.error.kind).toBe('bufferLength');
      expect(wrongLength.error.message).toContain('expected 8');
    }
    const empty = SourceImage.create(new Uint8Array(0), 0, 0);
    expect(empty.status === 'error' && empty.error.kind).toBe('emptyImage');
  });

  test('recolor writes into a Uint8Array', () => {
    const image = redAndTransparent();
    const out = new Uint8Array(8);
    const stats = unwrap(image.recolor(redToBlue, out));
    // Red is an exact source match; transparent is composited to white, and the only ink is blue.
    expect([...out]).toEqual([58, 117, 196, 255, 58, 117, 196, 255]);
    expect(stats).toEqual({ exact: 1, nearest: 1 });
  });

  test('recolor writes straight into an ImageData-style buffer through a view', () => {
    const image = redAndTransparent();
    const data = new Uint8ClampedArray(8); // what ImageData.data is
    unwrap(image.recolor(redToBlue, new Uint8Array(data.buffer)));
    expect([...data]).toEqual([58, 117, 196, 255, 58, 117, 196, 255]);
  });

  test('zero mappings give the composited copy; one mapping turns everything into its ink', () => {
    // I8 contract. Pixel 2 is fully transparent, so it composites to white.
    const image = redAndTransparent();
    const out = new Uint8Array(8);

    expect(unwrap(image.recolor({ mappings: [] }, out))).toEqual({ exact: 0, nearest: 2 });
    expect([...out]).toEqual([230, 76, 60, 255, 255, 255, 255, 255]);

    expect(unwrap(image.recolor(redToBlue, out))).toEqual({ exact: 1, nearest: 1 });
    expect([...out]).toEqual([58, 117, 196, 255, 58, 117, 196, 255]);
  });

  test('ties go to the earlier mapping (I9)', () => {
    // Exact matches: one source, two inks.
    const red = redAndTransparent();
    const out = new Uint8Array(8);
    const twoInks = (first: Rgb, second: Rgb): RecolorRequest => ({
      mappings: [
        { source: RED, ink: first },
        { source: RED, ink: second },
      ],
    });
    unwrap(red.recolor(twoInks(PANTONE_285, { r: 0, g: 0, b: 0 }), out));
    expect([...out.slice(0, 4)]).toEqual([58, 117, 196, 255]);
    unwrap(red.recolor(twoInks({ r: 0, g: 0, b: 0 }, PANTONE_285), out));
    expect([...out.slice(0, 4)]).toEqual([0, 0, 0, 255]);

    // Nearest-ink ties are not asserted here: native and WASM compute CIEDE2000 slightly
    // differently (about 1e-4), so a pair of inks that ties exactly on one platform doesn't tie on
    // the other. The exact-tie test is native-only, in `core` (`recolor` tests).
  });

  test('errors are values, and the module keeps working after them', () => {
    const image = redAndTransparent();
    const short = image.recolor(redToBlue, new Uint8Array(4));
    expect(short.status === 'error' && short.error.kind).toBe('outputLength');

    const malformed = [
      { mappings: 'x' },
      {},
      null,
      { mappings: [{ source: { r: 300, g: 0, b: 0 }, ink: PANTONE_285 }] },
      { mappings: [{ source: [230, 76, 60], ink: PANTONE_285 }] },
    ];
    for (const request of malformed) {
      const outcome = image.recolor(request as unknown as RecolorRequest, new Uint8Array(8));
      expect(outcome.status === 'error' && outcome.error.kind).toBe('invalidInput');
    }

    expect(unwrap(image.recolor(redToBlue, new Uint8Array(8)))).toEqual({ exact: 1, nearest: 1 });
  });

  test('pick reads image coordinates and reports a mismatch as a warning only', async () => {
    const palette = unwrap(Palette.create(await pantoneData()));
    const image = redAndTransparent();

    const pick = unwrap(image.pick(0, 0, undefined, palette));
    expect(pick.pixel).toEqual({ r: 230, g: 76, b: 60, a: 255 });
    expect(pick.matching).toEqual(RED);
    expect(pick.suggestion.name).toBe('Pantone 179');
    expect('mismatch' in pick).toBe(false);

    const transparent = unwrap(image.pick(1, 0, undefined, palette));
    expect(transparent.matching).toEqual({ r: 255, g: 255, b: 255 });

    // The color "seen" on screen is far off: still a valid pick, with a warning.
    const warned = unwrap(image.pick(0, 0, { r: 230, g: 76, b: 160, a: 255 }, palette));
    expect(warned.mismatch).toEqual({
      seen: { r: 230, g: 76, b: 160, a: 255 },
      stored: { r: 230, g: 76, b: 60, a: 255 },
      maxChannelDifference: 100,
    });
    expect(warned.pixel).toEqual(pick.pixel);

    const outside = image.pick(2, 0, undefined, palette);
    expect(outside.status === 'error' && outside.error.kind).toBe('outOfBounds');
    palette.free();
  });

  test('numbers that are not whole, finite, non-negative 32-bit values are error values', () => {
    // Review fix F1: wasm-bindgen would otherwise truncate/wrap them into valid-looking values.
    const one = new Uint8Array([5, 6, 7, 255]);
    for (const [w, h] of [[1.5, 1], [4294967297, 1], [NaN, 1], [Infinity, 1], [-1, 1], [1, 0.5]]) {
      const created = SourceImage.create(one, w!, h!);
      expect(created.status === 'error' && created.error.kind, `create(${w}, ${h})`).toBe('invalidInput');
    }

    const image = unwrap(SourceImage.create(one, 1, 1));
    const palette = unwrap(Palette.create({ entries: [{ name: 'X', rgb: { r: 0, g: 0, b: 0 } }] }));
    for (const [x, y] of [[NaN, 0], [Infinity, 0], [-0.9, 0], [4294967296, 0], [0.5, 0], [0, -1]]) {
      const picked = image.pick(x!, y!, undefined, palette);
      expect(picked.status === 'error' && picked.error.kind, `pick(${x}, ${y})`).toBe('invalidInput');
    }
    for (const k of [4294967296, 1.5, NaN, -1]) {
      const nearest = palette.nearest({ r: 0, g: 0, b: 0 }, k);
      expect(nearest.status === 'error' && nearest.error.kind, `nearest(k=${k})`).toBe('invalidInput');
    }

    // The same objects keep working.
    expect(unwrap(image.pick(0, 0, undefined, palette)).pixel).toEqual({ r: 5, g: 6, b: 7, a: 255 });
    expect(unwrap(palette.nearest({ r: 0, g: 0, b: 0 }, 1)).matches).toHaveLength(1);
  });

  test('free() releases the object; later calls throw', () => {
    const image = redAndTransparent();
    image.free();
    expect(() => image.analyze()).toThrow();
  });

  test('objects are disposable', () => {
    const image = redAndTransparent();
    expect(typeof image[Symbol.dispose]).toBe('function');
    image[Symbol.dispose]();
    expect(() => image.analyze()).toThrow();
  });
});
