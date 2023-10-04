import rgbHex from 'rgb-hex';

export function convertStringBase64ToU8Array(input: string): Uint8Array {
  input = atob(input);
  const result = new Uint8Array(input.length);
  for (let idx = 0; idx < input.length; idx++) {
    result[idx] = input.charCodeAt(idx);
  }
  return result;
}

export function convertU8ArrayToStringBase64(input: Uint8Array): string {
  return btoa(String.fromCharCode(...input));
}

export function rawFileToDataUrl(
  raw: Uint8Array | ArrayBuffer | string,
): string {
  if (typeof raw === 'string') {
    return raw;
  } else {
    return (
      'data:image/png;base64,' +
      convertU8ArrayToStringBase64(new Uint8Array(raw))
    );
  }
}

export function dataUrlToRawFile(input: string | ArrayBuffer): Uint8Array {
  if (typeof input === 'string') {
    const [, type, format, encoding, data] = input.match(
      /^data:(.+)\/(.+);(.+),(.*)$/,
    );
    console.log('dataUrlToRawFile', {
      type,
      format,
      encoding,
      data: `${data.length} bytes`,
    });
    console.assert(encoding == 'base64');
    return convertStringBase64ToU8Array(data);
  } else {
    return new Uint8Array(input);
  }
}

export function htmlColor({
  r,
  g,
  b,
  a,
}: {
  r: number;
  g: number;
  b: number;
  a?: number;
}): string {
  return `#${rgbHex(r, g, b, a)}`;
}

export function rgba2rgb({
  r,
  g,
  b,
  a,
}: {
  r: number;
  g: number;
  b: number;
  a: number;
}): { r: number; g: number; b: number } {
  a = a ?? 1;
  const factor = (1 - a) * 255;
  return {
    r: Math.round(factor + a * r),
    g: Math.round(factor + a * g),
    b: Math.round(factor + a * b),
  };
}
