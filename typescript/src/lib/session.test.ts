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
    expect(value(session.pick(b.generation, 0, 0)).pixel.r).toBe(200);
  });

  test('a failed replacement keeps the previous image and its generation', async () => {
    const { session, file } = setup();
    value(await session.open(file('a', decoded(100)), 1));
    expect(await session.open(file('junk'), 2)).toMatchObject({
      status: 'error',
      error: { kind: 'decodeFailed' },
    });
    expect(value(session.pick(1, 0, 0)).pixel.r).toBe(100);
    value(await session.recolor(1, 1, []));
    expect((await session.encodePng(1, 1)).status).toBe('ok');
  });

  test('a PNG that finishes encoding after a newer image opened is not returned', async () => {
    const { session, file, holdEncode } = setup();
    value(await session.open(file('a', decoded(100)), 1));
    value(await session.recolor(1, 1, []));
    const encoding = holdEncode();
    const png = session.encodePng(1, 1);
    value(await session.open(file('b', decoded(200)), 2));
    encoding.resolve(new Blob(['png of a']));
    expect(await png).toMatchObject({ status: 'error', error: { kind: 'superseded' } });
  });

  test('a PNG that finishes encoding after a newer recolor is not returned', async () => {
    const { session, file, holdEncode } = setup();
    value(await session.open(file('a', decoded(100)), 1));
    value(await session.recolor(1, 1, []));
    const encoding = holdEncode();
    const png = session.encodePng(1, 1);
    value(await session.recolor(1, 2, []));
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
      session.pick(1, 0, 0),
      session.colorCount(1),
      await session.recolor(1, 1, []),
      await session.encodePng(1, 1),
    ]) {
      expect(outcome).toMatchObject({ status: 'error', error: { kind: 'superseded' } });
    }
  });
});
