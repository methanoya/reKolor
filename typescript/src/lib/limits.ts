// Resource limits (owner decision WA1 a): checked before any pixel buffer is allocated.

export const LIMITS = {
  /** Encoded image file size. */
  fileBytes: 50 * 1024 * 1024,
  /** Width and height, each. */
  side: 16_384,
  /** Width × height: 24 megapixels, i.e. 96 MiB per RGBA buffer. */
  pixels: 24_000_000,
  /** Palette config import (W10 v). */
  configBytes: 256 * 1024,
  /** Picks (= recolor mappings), the same limit as a config section and WASM `recolor`. */
  picks: 256,
} as const;

/** Why an image of this size is refused, or `undefined` if it's within the limits. */
export function imageLimitError(width: number, height: number): string | undefined {
  if (!(width > 0 && height > 0)) return `the image is empty (${width}×${height})`;
  if (width > LIMITS.side || height > LIMITS.side) {
    return `the image is ${width}×${height}; each side can be at most ${LIMITS.side} pixels`;
  }
  // Both factors are ≤ 16,384, so the product is exact in a JS number.
  const pixels = width * height;
  if (pixels > LIMITS.pixels) {
    const mp = (n: number) => (n / 1_000_000).toFixed(1);
    return `the image has ${mp(pixels)} megapixels; at most ${mp(LIMITS.pixels)} are supported`;
  }
  return undefined;
}

export function fileLimitError(bytes: number): string | undefined {
  if (bytes > LIMITS.fileBytes) {
    const mb = (n: number) => (n / 1024 / 1024).toFixed(1);
    return `the file is ${mb(bytes)} MB; at most ${mb(LIMITS.fileBytes)} MB is supported`;
  }
  return undefined;
}
