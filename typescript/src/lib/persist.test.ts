// Unit tests for reading saved state (`persist.ts`, `parseSavedState`): what an earlier page of the
// app saved comes back as it was, and anything else (an older or hand-edited state) is ignored.
// The storage itself (sessionStorage, IndexedDB, Web Locks) is browser-only; the browser tests
// cover it.
import { afterEach, describe, expect, test, vi } from 'vitest';
import { LIMITS } from './limits';
import { loadState, parseSavedState, saveState, type SavedState } from './persist';

const ink = {
  index: 3,
  name: 'Pantone 179',
  rgb: { r: 226, g: 61, b: 40 },
  nonPalette: false,
  deltaE: 1.5,
};
const state: SavedState = {
  version: 1,
  image: { name: 'photo.png', size: 1234 },
  palette: { name: 'shop', text: '{"Shop Red": {"rgb": [220, 40, 40]}}' },
  material: { r: 10, g: 20, b: 30 },
  materialChosen: true,
  picks: [
    {
      id: 4,
      pixel: { r: 230, g: 76, b: 60, a: 255 },
      matching: { r: 230, g: 76, b: 60 },
      ink,
      alternatives: [ink],
      deltaE: 12,
      maxDeltaE: 40,
      at: { x: 5, y: 6 },
    },
  ],
  ranges: [
    { id: 2, pixel: { r: 10, g: 20, b: 30, a: 255 }, deltaE: 10, maxDeltaE: 40, material: true },
  ],
};

describe('parseSavedState', () => {
  test('reads back what was saved', () => {
    expect(parseSavedState(JSON.stringify(state))).toEqual(state);
    // Without an image or an uploaded palette.
    const minimal: SavedState = { ...state };
    delete minimal.image;
    delete minimal.palette;
    expect(parseSavedState(JSON.stringify(minimal))).toEqual(minimal);
  });

  test('ignores anything else', () => {
    const changed = (change: (s: Record<string, unknown>) => void) => {
      const copy = structuredClone(state) as unknown as Record<string, unknown>;
      change(copy);
      return parseSavedState(JSON.stringify(copy));
    };
    expect(parseSavedState('not json')).toBeUndefined();
    expect(parseSavedState('[]')).toBeUndefined();
    expect(changed((s) => (s.version = 2))).toBeUndefined();
    expect(changed((s) => (s.material = { r: 256, g: 0, b: 0 }))).toBeUndefined();
    expect(changed((s) => delete s.materialChosen)).toBeUndefined();
    expect(changed((s) => ((s.picks as { deltaE: number }[])[0]!.deltaE = 101))).toBeUndefined();
    expect(
      changed((s) => ((s.picks as { ink: unknown }[])[0]!.ink = { name: 'x' })),
    ).toBeUndefined();
    expect(changed((s) => ((s.ranges as { pixel: unknown }[])[0]!.pixel = 'red'))).toBeUndefined();
    expect(changed((s) => (s.palette = { name: 'shop' }))).toBeUndefined();
    expect(
      changed((s) => (s.picks = Array.from({ length: LIMITS.picks + 1 }, () => state.picks[0]))),
    ).toBeUndefined();
  });
});

describe('saveState', () => {
  afterEach(() => vi.unstubAllGlobals());

  test("a save that doesn't fit leaves the previous one", () => {
    // A stand-in for sessionStorage that refuses values over 2,000 characters, as a full one would.
    const items = new Map<string, string>();
    vi.stubGlobal('sessionStorage', {
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => {
        if (value.length > 2000) throw new DOMException('full', 'QuotaExceededError');
        items.set(key, value);
      },
      removeItem: (key: string) => items.delete(key),
    });
    saveState(state);
    expect(loadState()).toEqual(state);
    saveState({ ...state, palette: { name: 'big', text: 'x'.repeat(5000) } });
    expect(loadState()).toEqual(state);
  });
});
