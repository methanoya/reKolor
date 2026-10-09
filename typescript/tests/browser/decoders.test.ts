// The browser's decoding (the app's codec, `createImageBitmap` + canvas) compared with
// `rekolor-io`'s, per fixture:
// - opaque sRGB and EXIF PNG: exact size and pixels;
// - EXIF JPEG: exact size; colors close (JPEG decoders differ slightly);
// - alpha ramp: alpha exact, colors composited over white within ±1 (what recolor sees);
// - ICC profile: `rekolor-io` keeps the stored values and reports the profile; Chromium and WebKit
//   apply it, Firefox doesn't (see `README.md`). The size must match and the decoder warning must
//   be recorded; the color difference depends on the browser, so it is logged, not asserted.

// Browser tests (`npm run test:browser`), run in each browser engine; `server.browser` names the
// current one in the test titles. The reference pixels are the `.rgba` files written by the Rust
// side (see `fixtures.ts`).
import { server } from 'vitest/browser';
import { describe, expect, test } from 'vitest';
import { decodeImage } from '../../src/lib/codec';
import { fixtureFile, reference, rustPixels } from './fixtures';

async function browserDecode(name: string) {
  const decoded = await decodeImage(await fixtureFile(name));
  if (decoded.status === 'error')
    throw new Error(`${decoded.error.kind}: ${decoded.error.message}`);
  decoded.value.bitmap.close();
  return decoded.value;
}

/** Index of the first differing pixel, as "x,y: browser vs rust", or undefined. */
function firstDifference(a: Uint8Array, b: Uint8Array, width: number): string | undefined {
  for (let i = 0; i < a.length; i += 4) {
    if (a[i] !== b[i] || a[i + 1] !== b[i + 1] || a[i + 2] !== b[i + 2] || a[i + 3] !== b[i + 3]) {
      const p = i / 4;
      return `(${p % width}, ${Math.floor(p / width)}): browser ${[...a.subarray(i, i + 4)]} vs rust ${[...b.subarray(i, i + 4)]}`;
    }
  }
  return undefined;
}

// A channel composited over white, exactly as the Rust `composite` computes it:
// `(a·c + (255 − a)·255) / 255`, rounded down.
const overWhite = (c: number, a: number) => 255 - a + Math.floor((a * c) / 255);

describe(`browser decoding vs rekolor-io (${server.browser})`, () => {
  test('opaque sRGB PNG: exactly the same pixels', async () => {
    const ref = reference('opaque');
    const got = await browserDecode('opaque');
    expect([got.width, got.height]).toEqual([ref.width, ref.height]);
    expect(firstDifference(got.rgba, await rustPixels('opaque'), got.width)).toBeUndefined();
  });

  test('EXIF orientation in a PNG: same rotation, same pixels', async () => {
    const ref = reference('exif-6-png');
    expect(ref.warnings).toContain('OrientationApplied');
    const got = await browserDecode('exif-6-png');
    expect([got.width, got.height]).toEqual([ref.width, ref.height]);
    expect(firstDifference(got.rgba, await rustPixels('exif-6-png'), got.width)).toBeUndefined();
  });

  test('EXIF orientation in a JPEG: same rotation, colors within ±16', async () => {
    const ref = reference('exif-6-jpeg');
    const got = await browserDecode('exif-6-jpeg');
    expect([got.width, got.height]).toEqual([ref.width, ref.height]);
    const rust = await rustPixels('exif-6-jpeg');
    // Mean color of each quadrant's interior (4 px margin), which identifies the rotation.
    const mean = (px: Uint8Array, qx: number, qy: number) => {
      const w = got.width / 2;
      const h = got.height / 2;
      const sum = [0, 0, 0];
      let n = 0;
      for (let y = qy * h + 4; y < (qy + 1) * h - 4; y++) {
        for (let x = qx * w + 4; x < (qx + 1) * w - 4; x++) {
          for (let c = 0; c < 3; c++) sum[c]! += px[(y * got.width + x) * 4 + c]!;
          n++;
        }
      }
      return sum.map((s) => s / n);
    };
    for (const [qx, qy] of [
      [0, 0],
      [1, 0],
      [0, 1],
      [1, 1],
    ] as const) {
      const b = mean(got.rgba, qx, qy);
      const r = mean(rust, qx, qy);
      for (let c = 0; c < 3; c++) expect(Math.abs(b[c]! - r[c]!)).toBeLessThanOrEqual(16);
    }
  });

  test('alpha ramp: alpha exact; colors over white within ±1', async () => {
    const got = await browserDecode('alpha-ramp');
    const rust = await rustPixels('alpha-ramp');
    let worst = 0;
    for (let i = 0; i < rust.length; i += 4) {
      const a = rust[i + 3]!;
      expect(got.rgba[i + 3], `alpha at pixel ${i / 4}`).toBe(a);
      for (let c = 0; c < 3; c++) {
        const d = Math.abs(overWhite(got.rgba[i + c]!, a) - overWhite(rust[i + c]!, a));
        worst = Math.max(worst, d);
      }
    }
    expect(worst).toBeLessThanOrEqual(1);
  });

  test('ICC profile: same size and a reported warning; the color difference is logged', async () => {
    const ref = reference('icc-swapped');
    expect(ref.warnings).toContain('IccProfile');
    const got = await browserDecode('icc-swapped');
    expect([got.width, got.height]).toEqual([ref.width, ref.height]);
    const rust = await rustPixels('icc-swapped');
    let worst = 0;
    for (let i = 0; i < rust.length; i++)
      worst = Math.max(worst, Math.abs(got.rgba[i]! - rust[i]!));
    expect(worst).toBeGreaterThan(0);
    console.info(
      `[decoders] ${server.browser}: ICC-profiled PNG, largest channel difference ${worst}`,
    );
  });
});
