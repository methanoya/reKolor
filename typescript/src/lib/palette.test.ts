// Unit tests, run in Node.js by `npm test` (Vitest). `describe` groups tests, `test` defines one,
// and `expect(value).toEqual(...)` (or `.toThrow(...)`, …) checks a result; a failed check fails
// the test. A test file sits next to the module it tests (`palette.ts`).
import { describe, expect, test } from 'vitest';
import { parsePaletteJson } from './palette';
import { pantoneJson } from './test-wasm';

describe('parsePaletteJson', () => {
  test('keeps file order', async () => {
    const data = parsePaletteJson(await pantoneJson());
    expect(data.entries).toHaveLength(909);
    expect(data.entries.slice(0, 3).map((e) => e.name)).toEqual([
      'Pure White (non-palette)',
      'Pure Black (non-palette)',
      'Pantone 100',
    ]);
    expect(data.entries[2]!.rgb).toEqual({ r: 244, g: 237, b: 124 });
  });

  test('rejects malformed entries', () => {
    expect(() => parsePaletteJson('[]')).toThrow(/object/);
    expect(() => parsePaletteJson('{"a": {"rgb": [1, 2, 300]}}')).toThrow(/"a"/);
    expect(() => parsePaletteJson('{"a": {}}')).toThrow(/"a"/);
  });
});
