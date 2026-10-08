import { readFile } from 'node:fs/promises';
import { beforeAll, describe, expect, test } from 'vitest';
import { ALTERNATIVES, Engine } from './engine';
import type { AppOutcome } from './outcome';
import { loadWasm, pantoneJson } from './test-wasm';

const value = <T>(outcome: AppOutcome<T>): T => {
  if (outcome.status === 'error')
    throw new Error(`${outcome.error.kind}: ${outcome.error.message}`);
  return outcome.value;
};

let palette: string;
beforeAll(async () => {
  await loadWasm();
  palette = await pantoneJson();
});

/** 2×1: opaque red, then fully transparent. */
const redAndClear = () => new Uint8Array([230, 76, 60, 255, 9, 9, 9, 0]);

const WHITE = { r: 255, g: 255, b: 255 };
const BLACK = { r: 0, g: 0, b: 0 };

describe('Engine', () => {
  test('starts from the Pantone palette in file order', () => {
    const engine = value(Engine.create(palette));
    expect(engine.paletteSize).toBe(909);
    expect(engine.image).toBeUndefined();
    engine.dispose();
  });

  test('a broken palette is an initFailed error', () => {
    const outcome = Engine.create('{"x": {"rgb": [1, 2]}}');
    expect(outcome).toMatchObject({ status: 'error', error: { kind: 'initFailed' } });
  });

  test('calls without an image are noImage errors', () => {
    const engine = value(Engine.create(palette));
    for (const outcome of [
      engine.pick(0, 0, WHITE),
      engine.recolor([], WHITE),
      engine.analyze(WHITE),
      engine.colorCount(WHITE),
    ]) {
      expect(outcome).toMatchObject({ status: 'error', error: { kind: 'noImage' } });
    }
    engine.dispose();
  });

  test('open, pick, analyze and recolor', () => {
    const engine = value(Engine.create(palette));
    expect(value(engine.open(redAndClear(), 2, 1))).toEqual({ width: 2, height: 1 });

    const pick = value(engine.pick(1, 0, WHITE));
    expect(pick.pixel).toEqual({ r: 9, g: 9, b: 9, a: 0 });
    expect(pick.matching).toEqual({ r: 255, g: 255, b: 255 });
    expect(pick.suggestion.name).toBe('Pure White (non-palette)');

    expect(value(engine.analyze(WHITE))).toEqual({
      width: 2,
      height: 1,
      colors: 2,
      rgbaColors: 2,
    });
    expect(value(engine.colorCount(WHITE))).toBe(2);

    // Zero picks (WA4): the image as printed on white.
    expect([...value(engine.recolor([], WHITE))]).toEqual([230, 76, 60, 255, 255, 255, 255, 255]);
    const ink = { r: 58, g: 117, b: 196 };
    const out = value(engine.recolor([{ source: { r: 230, g: 76, b: 60 }, ink }], WHITE));
    expect([...out]).toEqual([58, 117, 196, 255, 58, 117, 196, 255]);
    engine.dispose();
  });

  test('a failed open keeps the previous image', () => {
    const engine = value(Engine.create(palette));
    value(engine.open(redAndClear(), 2, 1));
    expect(engine.open(new Uint8Array(3), 1, 1)).toMatchObject({
      status: 'error',
      error: { kind: 'bufferLength' },
    });
    expect(engine.image).toEqual({ width: 2, height: 1 });
    engine.dispose();
  });

  test('images over the limits are refused before WASM sees them', () => {
    const engine = value(Engine.create(palette));
    // The buffer is deliberately tiny: the size check must come first.
    for (const [w, h] of [
      [16_385, 1],
      [6_000, 5_000],
      [0, 1],
    ] as const) {
      expect(engine.open(new Uint8Array(4), w, h)).toMatchObject({
        status: 'error',
        error: { kind: 'imageTooLarge' },
      });
    }
    engine.dispose();
  });

  test('pick outside the image is an outOfBounds error', () => {
    const engine = value(Engine.create(palette));
    value(engine.open(redAndClear(), 2, 1));
    expect(engine.pick(2, 0, WHITE)).toMatchObject({
      status: 'error',
      error: { kind: 'outOfBounds' },
    });
    engine.dispose();
  });

  test(`nearest gives ${ALTERNATIVES} alternatives by default, nearest first`, () => {
    const engine = value(Engine.create(palette));
    const matches = value(engine.nearest({ r: 0, g: 63, b: 84 }));
    expect(matches).toHaveLength(ALTERNATIVES);
    // Same RGB: file order decides (I7).
    expect(matches.slice(0, 2).map((m) => m.name)).toEqual(['Pantone 303', 'Pantone 547']);
    expect(matches.map((m) => m.deltaE)).toEqual(
      [...matches.map((m) => m.deltaE)].sort((a, b) => a - b),
    );
    engine.dispose();
  });

  test('close frees the image', () => {
    const engine = value(Engine.create(palette));
    value(engine.open(redAndClear(), 2, 1));
    engine.close();
    expect(engine.image).toBeUndefined();
    expect(engine.pick(0, 0, WHITE)).toMatchObject({ status: 'error', error: { kind: 'noImage' } });
    engine.dispose();
  });

  test('every call composites over the material it is given', () => {
    const engine = value(Engine.create(palette));
    value(engine.open(redAndClear(), 2, 1));
    const pick = value(engine.pick(1, 0, BLACK));
    expect(pick.matching).toEqual(BLACK);
    expect(pick.suggestion.name).toBe('Pure Black (non-palette)');
    expect(value(engine.pick(0, 0, BLACK)).matching).toEqual({ r: 230, g: 76, b: 60 });
    // Zero picks: the image as printed on the material.
    expect([...value(engine.recolor([], BLACK))]).toEqual([230, 76, 60, 255, 0, 0, 0, 255]);
    expect(value(engine.analyze(BLACK)).colors).toBe(2);
    expect(value(engine.composite({ r: 203, g: 0, b: 0, a: 100 }, BLACK))).toEqual({
      r: 79,
      g: 0,
      b: 0,
    });
    engine.dispose();
  });
});

describe('Engine configs (W10 v)', () => {
  const sample = () =>
    readFile(
      new URL('../../../samples/others/icon-calendar.palettes.toml', import.meta.url),
      'utf8',
    );

  test('import a golden-set config section and export it again', async () => {
    const engine = value(Engine.create(palette));
    const parsed = value(engine.parseConfig(await sample()));
    expect(parsed.sections.map((s) => s.size)).toEqual([3, 7, 16]);
    expect(parsed.material).toEqual(WHITE); // no `material` line: white (M4.1 a)
    const section = parsed.sections[1]!;
    const resolved = value(engine.resolveSection(section, parsed.material));
    expect(resolved.map((p) => p.ink.name)).toEqual(section.picks.map((p) => p.ink));

    const text = value(
      engine.exportConfig(
        'icon-calendar.png',
        parsed.material,
        resolved.map((p) => ({ rgba: p.pixel, ink: p.ink.name })),
      ),
    );
    expect(value(engine.parseConfig(text))).toEqual({
      material: WHITE,
      sections: [{ size: section.picks.length, picks: section.picks }],
    });
    engine.dispose();
  });

  test('unknown inks and broken files are invalidConfig errors', () => {
    const engine = value(Engine.create(palette));
    expect(engine.parseConfig('[[palette]]\nsize = 1\n')).toMatchObject({
      status: 'error',
      error: { kind: 'invalidConfig' },
    });
    expect(
      engine.resolveSection(
        { size: 1, picks: [{ rgba: { r: 0, g: 0, b: 0, a: 255 }, ink: 'Not an ink' }] },
        WHITE,
      ),
    ).toMatchObject({
      status: 'error',
      error: { kind: 'invalidConfig', message: expect.stringContaining('Not an ink') },
    });
    engine.dispose();
  });
});
