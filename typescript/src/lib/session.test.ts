// The worker's rules with a controllable fake codec: the races from the code review, made
// deterministic (a slow decode, an open while a PNG encodes, a failed replacement).

import { beforeAll, describe, expect, test, vi } from 'vitest';
import type { Decoded } from './codec';
import { Engine } from './engine';
import { err, ok, type AppOutcome } from './outcome';
import { Session, type Codec } from './session';
import { loadWasm, pantoneJson } from './test-wasm';

let palette: string;
beforeAll(async () => {
  await loadWasm();
  palette = await pantoneJson();
});

const value = <T>(outcome: AppOutcome<T>): T => {
  if (outcome.status === 'error')
    throw new Error(`${outcome.error.kind}: ${outcome.error.message}`);
  return outcome.value;
};

/** A promise the test resolves when it wants. */
function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

const fakeBitmap = () => ({ close: vi.fn() }) as unknown as ImageBitmap;

const WHITE = { r: 255, g: 255, b: 255 };
const BLACK = { r: 0, g: 0, b: 0 };

/** A decoded 2×1 image whose first pixel is `r, 0, 0`. */
const decoded = (r: number): AppOutcome<Decoded> =>
  ok({
    rgba: new Uint8Array([r, 0, 0, 255, 9, 9, 9, 255]),
    width: 2,
    height: 1,
    bitmap: fakeBitmap(),
  });

/** A fake codec: files are strings naming what decode should return; decode and encode can be held. */
function setup() {
  const decodes = new Map<string, Promise<AppOutcome<Decoded>>>();
  let encode: Promise<Blob> | undefined;
  const codec: Codec = {
    decode: async (file) => decodes.get(await file.text()) ?? err('decodeFailed', 'junk'),
    toBitmap: async () => fakeBitmap(),
    encodePng: () => encode ?? Promise.resolve(new Blob(['png'])),
  };
  const session = new Session(value(Engine.create(palette)), codec);
  return {
    session,
    file: (name: string, result?: AppOutcome<Decoded> | Promise<AppOutcome<Decoded>>) => {
      if (result) decodes.set(name, Promise.resolve(result));
      return new Blob([name]);
    },
    holdEncode: () => {
      const d = deferred<Blob>();
      encode = d.promise;
      return d;
    },
  };
}

describe('Session (the worker rules)', () => {
  test('a slow older decode does not replace a newer image', async () => {
    const { session, file } = setup();
    const slow = deferred<AppOutcome<Decoded>>();
    const a = session.open(file('a', slow.promise), 1);
    const b = value(await session.open(file('b', decoded(200)), 2));
    const aDecoded = decoded(100);
    slow.resolve(aDecoded);
    expect(await a).toMatchObject({ status: 'error', error: { kind: 'superseded' } });
    // A's bitmap was closed; B is the open image.
    expect(
      (aDecoded.status === 'ok' && aDecoded.value.bitmap.close) as () => void,
    ).toHaveBeenCalled();
    expect(value(session.pick(b.generation, 0, 0, WHITE)).pixel.r).toBe(200);
  });

  test('a failed replacement keeps the previous image and its generation', async () => {
    const { session, file } = setup();
    value(await session.open(file('a', decoded(100)), 1));
    expect(await session.open(file('junk'), 2)).toMatchObject({
      status: 'error',
      error: { kind: 'decodeFailed' },
    });
    expect(value(session.pick(1, 0, 0, WHITE)).pixel.r).toBe(100);
    value(await session.recolor(1, 1, [], WHITE));
    expect((await session.encodePng(1, 1)).status).toBe('ok');
  });

  test('a PNG that finishes encoding after a newer image opened is not returned', async () => {
    const { session, file, holdEncode } = setup();
    value(await session.open(file('a', decoded(100)), 1));
    value(await session.recolor(1, 1, [], WHITE));
    const encoding = holdEncode();
    const png = session.encodePng(1, 1);
    value(await session.open(file('b', decoded(200)), 2));
    encoding.resolve(new Blob(['png of a']));
    expect(await png).toMatchObject({ status: 'error', error: { kind: 'superseded' } });
  });

  test('a PNG that finishes encoding after a newer recolor is not returned', async () => {
    const { session, file, holdEncode } = setup();
    value(await session.open(file('a', decoded(100)), 1));
    value(await session.recolor(1, 1, [], WHITE));
    const encoding = holdEncode();
    const png = session.encodePng(1, 1);
    value(await session.recolor(1, 2, [], WHITE));
    encoding.resolve(new Blob(['png of revision 1']));
    expect(await png).toMatchObject({ status: 'error', error: { kind: 'superseded' } });
    // The current revision (2) still encodes.
    expect((await session.encodePng(1, 2)).status).toBe('ok');
  });

  test('calls for an older generation are superseded', async () => {
    const { session, file } = setup();
    value(await session.open(file('a', decoded(100)), 1));
    value(await session.open(file('b', decoded(200)), 2));
    for (const outcome of [
      session.pick(1, 0, 0, WHITE),
      session.colorCount(1, WHITE),
      await session.recolor(1, 1, [], WHITE),
      await session.encodePng(1, 1),
    ]) {
      expect(outcome).toMatchObject({ status: 'error', error: { kind: 'superseded' } });
    }
  });
});

describe('Session.rematch (material changes)', () => {
  test('re-matches each pick in Rust; only changed colors get new inks', () => {
    const { session } = setup();
    const opaque = { pixel: { r: 230, g: 76, b: 60, a: 255 }, matching: { r: 230, g: 76, b: 60 } };
    const clear = { pixel: { r: 9, g: 9, b: 9, a: 0 }, matching: WHITE };
    const half = { pixel: { r: 203, g: 0, b: 0, a: 100 }, matching: { r: 234, g: 155, b: 155 } };

    const [a, b, c] = value(session.rematch([opaque, clear, half], BLACK));
    expect(a).toEqual({ matching: opaque.matching });
    expect(b!.matching).toEqual(BLACK);
    expect(b!.alternatives).toHaveLength(8);
    expect(b!.alternatives![0]!.name).toBe('Pure Black (non-palette)');
    expect(c!.matching).toEqual({ r: 79, g: 0, b: 0 });
    expect(c!.alternatives![0]).toEqual(value(session.nearest({ r: 79, g: 0, b: 0 }, 1))[0]);

    // Back on white: nothing changed for picks already matched on white.
    expect(value(session.rematch([opaque, clear, half], WHITE))).toEqual([
      { matching: opaque.matching },
      { matching: WHITE },
      { matching: half.matching },
    ]);
  });
});
