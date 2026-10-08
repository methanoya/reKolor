// Browser-side image codec (the browser decodes for the app). Works in a worker or on the main
// thread: `createImageBitmap` + `OffscreenCanvas`. Tested in real browsers (tests/browser).

import { fileLimitError, imageLimitError } from './limits';
import { err, ok, type AppOutcome } from './outcome';

export interface Decoded {
  /** Straight-alpha RGBA, row-major, sRGB. */
  rgba: Uint8Array<ArrayBuffer>;
  width: number;
  height: number;
  /** The decoded image, for display. The caller owns it (close it when done). */
  bitmap: ImageBitmap;
}

/**
 * Decodes an image file: EXIF orientation applied, the embedded color profile converted to sRGB,
 * straight (not premultiplied) alpha in the result. The size limits are checked before any
 * pixel buffer is made.
 *
 * Canvases store premultiplied alpha internally, so semi-transparent colors can come back slightly
 * changed; the cross-decoder test measures this.
 */
export async function decodeImage(file: Blob): Promise<AppOutcome<Decoded>> {
  const fileTooLarge = fileLimitError(file.size);
  if (fileTooLarge) return err('fileTooLarge', fileTooLarge);

  let bitmap: ImageBitmap;
  try {
    // `premultiplyAlpha` stays at its default on purpose: the canvas stores premultiplied alpha
    // anyway, and WebKit mishandles a `'none'` bitmap drawn onto a canvas (it un-premultiplies
    // twice: alpha 117, red 117 came back as red 255). Found by the cross-decoder test.
    bitmap = await createImageBitmap(file, {
      imageOrientation: 'from-image',
      colorSpaceConversion: 'default',
    });
  } catch (e) {
    return err('decodeFailed', `the browser couldn't decode this file (${(e as Error).message})`);
  }
  const { width, height } = bitmap;
  const tooLarge = imageLimitError(width, height);
  if (tooLarge) {
    bitmap.close();
    return err('imageTooLarge', tooLarge);
  }
  try {
    const context = new OffscreenCanvas(width, height).getContext('2d', {
      willReadFrequently: true,
    });
    if (!context) throw new Error('no 2D canvas context');
    context.drawImage(bitmap, 0, 0);
    const data = context.getImageData(0, 0, width, height, { colorSpace: 'srgb' }).data;
    return ok({ rgba: new Uint8Array(data.buffer), width, height, bitmap });
  } catch (e) {
    bitmap.close();
    return err('decodeFailed', `reading the decoded pixels failed (${(e as Error).message})`);
  }
}

const imageData = (rgba: Uint8Array<ArrayBuffer>, width: number, height: number) =>
  new ImageData(
    new Uint8ClampedArray(rgba.buffer, rgba.byteOffset, rgba.byteLength),
    width,
    height,
  );

/** An `ImageBitmap` of RGBA pixels, for display. */
export function toBitmap(
  rgba: Uint8Array<ArrayBuffer>,
  width: number,
  height: number,
): Promise<ImageBitmap> {
  return createImageBitmap(imageData(rgba, width, height));
}

/** A PNG file of RGBA pixels (full resolution). */
export function encodePng(
  rgba: Uint8Array<ArrayBuffer>,
  width: number,
  height: number,
): Promise<Blob> {
  const canvas = new OffscreenCanvas(width, height);
  const context = canvas.getContext('2d');
  if (!context) throw new Error('no 2D canvas context');
  context.putImageData(imageData(rgba, width, height), 0, 0);
  return canvas.convertToBlob({ type: 'image/png' });
}
