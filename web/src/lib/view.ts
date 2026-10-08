// Zoom and pan (W10 iv a): one view shared by the original and the result. Pure math, unit-tested.
//
// A view maps image coordinates to the viewport's CSS pixels:
//   css = (image − origin) × scale,   image = origin + css ÷ scale
// where `x`, `y` is the image coordinate at the viewport's top-left corner.

export interface Size {
  width: number;
  height: number;
}

export interface View {
  scale: number;
  x: number;
  y: number;
}

export const MIN_SCALE = 1 / 64;
export const MAX_SCALE = 64;

const clampScale = (s: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, s));

/** The whole image, centered (never enlarged beyond 1:1 for small images). */
export function fit(image: Size, viewport: Size): View {
  const scale = clampScale(
    Math.min(1, viewport.width / image.width, viewport.height / image.height),
  );
  return centered(image, viewport, scale);
}

/** The image centered at a given scale. */
export function centered(image: Size, viewport: Size, scale: number): View {
  const s = clampScale(scale);
  return {
    scale: s,
    x: (image.width - viewport.width / s) / 2,
    y: (image.height - viewport.height / s) / 2,
  };
}

/** Zooms by `factor`, keeping the image point under the CSS position (px, py) where it is. */
export function zoomAt(view: View, factor: number, px: number, py: number): View {
  const scale = clampScale(view.scale * factor);
  const ix = view.x + px / view.scale;
  const iy = view.y + py / view.scale;
  return { scale, x: ix - px / scale, y: iy - py / scale };
}

/** Moves the image by (dx, dy) CSS pixels (dragging). */
export function pan(view: View, dx: number, dy: number): View {
  return { ...view, x: view.x - dx / view.scale, y: view.y - dy / view.scale };
}

/** Keeps the image point at the viewport's center when the viewport changes size. */
export function resize(view: View, from: Size, to: Size): View {
  const cx = view.x + from.width / 2 / view.scale;
  const cy = view.y + from.height / 2 / view.scale;
  return { ...view, x: cx - to.width / 2 / view.scale, y: cy - to.height / 2 / view.scale };
}

/** The source pixel under a CSS position, or `undefined` outside the image. */
export function imagePixel(
  view: View,
  px: number,
  py: number,
  image: Size,
): { x: number; y: number } | undefined {
  const x = Math.floor(view.x + px / view.scale);
  const y = Math.floor(view.y + py / view.scale);
  return x >= 0 && y >= 0 && x < image.width && y < image.height ? { x, y } : undefined;
}

/** The CSS position of a source pixel's center. */
export function pixelCenter(view: View, x: number, y: number): { px: number; py: number } {
  return { px: (x + 0.5 - view.x) * view.scale, py: (y + 0.5 - view.y) * view.scale };
}
