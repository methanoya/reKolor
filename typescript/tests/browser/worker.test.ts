// The real worker through Comlink (T7, W6 b): decoding in the worker, generations, transfers,
// typed errors, and the PNG of exactly the current revision.

import { afterAll, describe, expect, test } from 'vitest';
import { EngineClient } from '../../src/lib/client';
import type { AppOutcome } from '../../src/lib/outcome';
import { fixtureFile, reference, rustPixels } from './fixtures';

const value = <T>(outcome: AppOutcome<T>): T => {
  if (outcome.status === 'error')
    throw new Error(`${outcome.error.kind}: ${outcome.error.message}`);
  return outcome.value;
};

const restarts: string[] = [];
const client = new EngineClient((reason) => restarts.push(reason));
afterAll(() => client.dispose());

describe('the engine worker', () => {
  test('starts with the Pantone palette', async () => {
    expect(value(await client.call((api) => api.ready()))).toEqual({ paletteSize: 909 });
  });

  test('opens a file, picks the stored pixel, recolors and encodes the current revision', async () => {
    const file = await fixtureFile('opaque');
    const opened = value(await client.call((api) => api.open(file, 1)));
    expect([opened.width, opened.height]).toEqual([
      reference('opaque').width,
      reference('opaque').height,
    ]);
    expect(opened.bitmap).toBeInstanceOf(ImageBitmap);
    opened.bitmap.close();

    const rust = await rustPixels('opaque');
    const [r, g, b] = rust.subarray((5 * 48 + 7) * 4);
    const pick = value(await client.call((api) => api.pick(1, 7, 5)));
    expect(pick.pixel).toEqual({ r, g, b, a: 255 });

    const ink = { r: 58, g: 117, b: 196 };
    const recolored = value(
      await client.call((api) => api.recolor(1, 1, [{ source: pick.matching, ink }])),
    );
    expect([recolored.generation, recolored.revision]).toEqual([1, 1]);
    recolored.bitmap.close();

    const png = value(await client.call((api) => api.encodePng(1, 1)));
    expect(png.type).toBe('image/png');
    // One mapping: every pixel takes that ink (I8), so the PNG decodes to a single color.
    const bitmap = await createImageBitmap(png);
    const ctx = new OffscreenCanvas(bitmap.width, bitmap.height).getContext('2d')!;
    ctx.drawImage(bitmap, 0, 0);
    const px = ctx.getImageData(0, 0, bitmap.width, bitmap.height).data;
    expect([px[0], px[1], px[2], px[3]]).toEqual([58, 117, 196, 255]);
    expect(new Set(new Uint32Array(px.buffer)).size).toBe(1);

    // A different revision than the one recolored: never encoded.
    expect(await client.call((api) => api.encodePng(1, 2))).toMatchObject({
      status: 'error',
      error: { kind: 'superseded' },
    });
  });

  test('a newer generation supersedes older calls', async () => {
    const file = await fixtureFile('opaque');
    value(await client.call((api) => api.open(file, 2))).bitmap.close();
    for (const outcome of [
      await client.call((api) => api.pick(1, 0, 0)),
      await client.call((api) => api.recolor(1, 9, [])),
      await client.call((api) => api.encodePng(1, 1)),
    ]) {
      expect(outcome).toMatchObject({ status: 'error', error: { kind: 'superseded' } });
    }
  });

  test('when two opens race, the older decode is dropped', async () => {
    const [a, b] = await Promise.all([
      client.call(async (api) => api.open(await fixtureFile('alpha-ramp'), 3)),
      client.call(async (api) => api.open(await fixtureFile('opaque'), 4)),
    ]);
    // Generation 4 is the newest requested; generation 3 is either superseded (finished later)
    // or was committed first and then replaced. Either way the open image is generation 4's.
    if (a.status === 'ok') a.value.bitmap.close();
    else expect(a.error.kind).toBe('superseded');
    value(b).bitmap.close();
    expect(value(await client.call((api) => api.pick(4, 47, 31))).pixel.a).toBe(255);
    expect(await client.call((api) => api.pick(3, 0, 0))).toMatchObject({ status: 'error' });
  });

  test('undecodable files and oversized configs are typed errors; the worker keeps working', async () => {
    const junk = new File([new Uint8Array([1, 2, 3, 4, 5])], 'junk.png', { type: 'image/png' });
    expect(await client.call((api) => api.open(junk, 5))).toMatchObject({
      status: 'error',
      error: { kind: 'decodeFailed' },
    });
    expect(await client.call((api) => api.parseConfig('x'.repeat(300 * 1024)))).toMatchObject({
      status: 'error',
      error: { kind: 'invalidConfig' },
    });
    expect(value(await client.call((api) => api.ready())).paletteSize).toBe(909);
    expect(restarts).toEqual([]);
  });
  // The encode/open race is tested deterministically in src/lib/session.test.ts (fake codec):
  // here the timing depends on the engine (Chromium and WebKit run the decode after the encode).
});
