import { diff } from 'color-diff';
import type { RgbColor8 } from './types';
import pantonePalette from './palettes/pantone.json';

export function pantone(original: RgbColor8): [RgbColor8, string] {
  let bestPaletteColor: string,
    bestOutColor: string,
    bestPaletteDistance: number = Number.MAX_VALUE,
    bestOutDistance: number = Number.MAX_VALUE;
  for (const name in pantonePalette) {
    const {
      rgb: [R, G, B],
    } = pantonePalette[name];
    const distance = diff(original, {
      R,
      G,
      B,
    });
    // console.log(original.r, original.g, original.b, 'to', name, distance);
    if (name.indexOf('non-palette') >= 0) {
      if (distance < bestOutDistance) {
        bestOutColor = name;
        bestOutDistance = distance;
      }
    } else {
      if (distance < bestPaletteDistance) {
        bestPaletteColor = name;
        bestPaletteDistance = distance;
      }
    }
  }
  console.log({
    bestPaletteColor,
    bestPaletteDistance,
    bestOutColor,
    bestOutDistance,
  });
  const bestColor =
    bestPaletteDistance < bestOutDistance * 1.5
      ? bestPaletteColor
      : bestOutColor;
  const { rgb } = pantonePalette[bestColor];
  console.log('best color is', bestColor, rgb);
  {
    const [r, g, b] = rgb;
    return [
      {
        r,
        g,
        b,
      },
      bestColor,
    ];
  }
}
