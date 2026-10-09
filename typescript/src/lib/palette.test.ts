// Unit tests, run in Node.js by `npm test` (Vitest). `describe` groups tests, `test` defines one,
// and `expect(value).toEqual(...)` (or `.toThrow(...)`, …) checks a result; a failed check fails
// the test. A test file sits next to the module it tests (`palette.ts`).
import { readFile } from 'node:fs/promises';
import { Ajv2020 } from 'ajv/dist/2020.js';
import { describe, expect, test } from 'vitest';
import { LIMITS } from './limits';
import { parsePaletteJson } from './palette';
import { pantoneJson } from './test-wasm';

/** The palette format's JSON Schema (`palettes/palette.schema.json`), compiled by Ajv. */
async function schemaValidator() {
  const schema: unknown = JSON.parse(
    await readFile(new URL('../../../palettes/palette.schema.json', import.meta.url), 'utf8'),
  );
  return new Ajv2020({ allErrors: true }).compile(schema as object);
}

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
    expect(() => parsePaletteJson('{}')).toThrow(/at least one/);
    expect(() => parsePaletteJson('{"": {"rgb": [1, 2, 3]}}')).toThrow(/name/);
  });

  test(`refuses more than ${LIMITS.paletteInks} inks`, () => {
    const inks = (n: number) =>
      JSON.stringify(
        Object.fromEntries(Array.from({ length: n }, (_, i) => [`ink ${i}`, { rgb: [1, 2, 3] }])),
      );
    expect(parsePaletteJson(inks(LIMITS.paletteInks)).entries).toHaveLength(LIMITS.paletteInks);
    expect(() => parsePaletteJson(inks(LIMITS.paletteInks + 1))).toThrow(/at most/);
  });
});

describe('the palette JSON Schema', () => {
  test('accepts the built-in palette and its own example', async () => {
    const valid = await schemaValidator();
    expect(valid(JSON.parse(await pantoneJson()))).toBe(true);
    const schema = JSON.parse(
      await readFile(new URL('../../../palettes/palette.schema.json', import.meta.url), 'utf8'),
    ) as { examples: unknown[] };
    for (const example of schema.examples) expect(valid(example)).toBe(true);
  });

  // The schema and the app's reader must agree on what a palette is.
  test('agrees with the reader on valid and invalid palettes', async () => {
    const valid = await schemaValidator();
    const accepted = ['{"a": {"rgb": [1, 2, 3]}}', '{"a": {"rgb": [0, 0, 0], "note": "ignored"}}'];
    const refused = [
      '[]',
      '"red"',
      '{}',
      '{"": {"rgb": [1, 2, 3]}}',
      '{"a": {}}',
      '{"a": {"rgb": [1, 2]}}',
      '{"a": {"rgb": [1, 2, 3, 4]}}',
      '{"a": {"rgb": [1, 2, 300]}}',
      '{"a": {"rgb": [1, 2, -1]}}',
      '{"a": {"rgb": [1.5, 2, 3]}}',
      '{"a": {"rgb": "red"}}',
      '{"a": [1, 2, 3]}',
    ];
    for (const text of accepted) {
      expect(valid(JSON.parse(text)), text).toBe(true);
      expect(() => parsePaletteJson(text), text).not.toThrow();
    }
    for (const text of refused) {
      expect(valid(JSON.parse(text)), text).toBe(false);
      expect(() => parsePaletteJson(text), text).toThrow();
    }
  });
});
