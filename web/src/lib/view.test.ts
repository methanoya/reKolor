import { describe, expect, test } from 'vitest';
import { centered, fit, imagePixel, pan, pixelCenter, resize, zoomAt } from './view';

const image = { width: 400, height: 200 };
const viewport = { width: 200, height: 200 };

describe('view', () => {
  test('fit shows the whole image, centered, never enlarged', () => {
    expect(fit(image, viewport)).toEqual({ scale: 0.5, x: 0, y: -100 });
    expect(fit({ width: 10, height: 10 }, viewport)).toEqual({ scale: 1, x: -95, y: -95 });
  });

  test('clicks land on the source pixel at any zoom', () => {
    for (const scale of [0.25, 0.5, 1, 3, 8, 37.5]) {
      const view = centered(image, viewport, scale);
      for (const [x, y] of [
        [0, 0],
        [123, 45],
        [399, 199],
      ] as const) {
        const { px, py } = pixelCenter(view, x, y);
        expect(imagePixel(view, px, py, image)).toEqual({ x, y });
      }
    }
  });

  test('positions outside the image pick nothing', () => {
    const view = fit(image, viewport); // letterboxed: rows above and below the image
    expect(imagePixel(view, 100, 10, image)).toBeUndefined();
    expect(imagePixel(view, 100, 190, image)).toBeUndefined();
    expect(imagePixel(view, 100, 100, image)).toEqual({ x: 200, y: 100 });
  });

  test('zoomAt keeps the point under the pointer fixed', () => {
    const view = fit(image, viewport);
    const before = imagePixel(view, 37, 120, image);
    const zoomed = zoomAt(view, 4, 37, 120);
    expect(zoomed.scale).toBe(2);
    expect(imagePixel(zoomed, 37, 120, image)).toEqual(before);
  });

  test('zoom is clamped', () => {
    expect(zoomAt(centered(image, viewport, 1), 1e6, 0, 0).scale).toBe(64);
    expect(zoomAt(centered(image, viewport, 1), 1e-6, 0, 0).scale).toBe(1 / 64);
  });

  test('pan moves the image with the pointer', () => {
    const view = centered(image, viewport, 2);
    const moved = pan(view, 20, -10);
    const p = pixelCenter(view, 150, 100);
    expect(imagePixel(moved, p.px + 20, p.py - 10, image)).toEqual({ x: 150, y: 100 });
  });

  test('resizing (and device pixel ratio changes) keep the centered source pixel', () => {
    const view = centered(image, viewport, 3);
    const center = imagePixel(view, 100, 100, image);
    const wider = resize(view, viewport, { width: 600, height: 300 });
    expect(imagePixel(wider, 300, 150, image)).toEqual(center);
  });
});
