// Records the 2023 JavaScript Pantone suggestion (the `pantone()` function from
// typescript/src/palette.ts, verbatim apart from logging) for a fixed set of colors, as the
// reference for characterizing the Rust suggestion (R10).
//
//     node rust/testdata/record_pantone_suggestions.cjs
//
// Needs `color-diff` from typescript/node_modules (installed by `npm ci` in typescript/).
// Writes rust/testdata/baseline/pantone-suggestions.tsv.

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..', '..');
const { diff, rgb_to_lab } = require(path.join(root, 'typescript/node_modules/color-diff'));
const pantonePalette = JSON.parse(fs.readFileSync(path.join(root, 'palettes/pantone.json'), 'utf8'));

// --- from typescript/src/palette.ts (2023) ---
function pantone(original) {
  let bestPaletteColor,
    bestOutColor,
    bestPaletteDistance = Number.MAX_VALUE,
    bestOutDistance = Number.MAX_VALUE;
  for (const name in pantonePalette) {
    const {
      rgb: [R, G, B],
    } = pantonePalette[name];
    const { r, g, b } = original;
    const distance = diff(rgb_to_lab({ R: r, G: g, B: b }), rgb_to_lab({ R, G, B }));
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
  const bestColor = bestPaletteDistance < bestOutDistance * 1.5 ? bestPaletteColor : bestOutColor;
  const distance = bestColor === bestPaletteColor ? bestPaletteDistance : bestOutDistance;
  return [bestColor, distance];
}
// --- end of 2023 code ---

// Deterministic color set: a 16-level grid (0, 17, ..., 255) plus 1000 pseudo-random colors.
const colors = [];
for (let r = 0; r <= 255; r += 17)
  for (let g = 0; g <= 255; g += 17)
    for (let b = 0; b <= 255; b += 17) colors.push([r, g, b]);
let seed = 12345;
const next = () => (seed = (Math.imul(seed, 1103515245) + 12345) & 0x7fffffff) & 255;
for (let i = 0; i < 1000; i++) colors.push([next(), next(), next()]);

let tsv = '# r\tg\tb\tsuggestion\tdelta_e_2023_js\n';
for (const [r, g, b] of colors) {
  const [name, distance] = pantone({ r, g, b });
  tsv += `${r}\t${g}\t${b}\t${name}\t${distance.toFixed(6)}\n`;
}
const out = path.join(__dirname, 'baseline', 'pantone-suggestions.tsv');
fs.writeFileSync(out, tsv);
console.log(`${colors.length} suggestions written to ${path.relative(root, out)}`);
