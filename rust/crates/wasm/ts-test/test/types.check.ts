// Type-level contract checks: `tsc --noEmit` must accept every line, and each
// `@ts-expect-error` must really be an error. Never run, only type-checked.

import type {
  ConfigSection,
  ConfigUnprinted,
  ErrorKind,
  ImageStats,
  MaterialRange,
  Outcome,
  Pick,
  RecolorStats,
  Rgb,
  SourceImage,
} from 'rekolor-wasm';

declare const outcome: Outcome<RecolorStats>;
declare const stats: ImageStats;
declare const image: SourceImage;
declare const pick: Pick;
declare const white: Rgb;

// The value is only reachable after narrowing on `status` (T5).
// @ts-expect-error
outcome.value;
if (outcome.status === 'ok') {
  const exact: number = outcome.value.exact;
  void exact;
} else {
  const kind: ErrorKind = outcome.error.kind;
  void kind;
}

// Fields are camelCase on the TypeScript side (T6).
const colors: number = stats.colors;
void colors;
// @ts-expect-error
stats.rgb_colors;
// The old field name is gone (I4).
// @ts-expect-error
stats.rgbColors;

// Pixel buffers are typed arrays, never plain arrays (T3).
// @ts-expect-error
image.recolor({ mappings: [], material: white }, [0, 0, 0, 0]);

// Colors are { r, g, b } objects.
// @ts-expect-error
image.recolor({ mappings: [{ source: [1, 2, 3], ink: { r: 0, g: 0, b: 0 } }], material: white }, new Uint8Array(4));

// The material is required everywhere it is used (no hidden white default).
// @ts-expect-error
image.recolor({ mappings: [] }, new Uint8Array(4));
// @ts-expect-error
image.colorCount();
// @ts-expect-error
image.analyze();

// The mismatch warning is optional (present only when there is one).
const mismatch: Pick['mismatch'] = undefined;
void mismatch;
if (pick.mismatch) {
  const difference: number = pick.mismatch.maxChannelDifference;
  void difference;
}

// Error kinds are a closed set.
const known: ErrorKind = 'outOfBounds';
void known;
// @ts-expect-error
const unknownKind: ErrorKind = 'oops';
void unknownKind;

// The palette argument of `pick` is required (there is no default palette).
// @ts-expect-error
image.pick(0, 0, undefined, white);

// Config picks are { rgba: Rgba, ink: string }; the size is a number.
const section: ConfigSection = { size: 1, picks: [{ rgba: { r: 1, g: 2, b: 3, a: 4 }, ink: 'x' }] };
void section;
// @ts-expect-error
const badSection: ConfigSection = { size: 1, picks: [{ rgba: [1, 2, 3, 4], ink: 'x' }] };
void badSection;
const configKind: ErrorKind = 'invalidConfig';
void configKind;

// A material range is a stored pixel (RGBA) and a ΔE; `materialRanges` is optional.
const range: MaterialRange = { pixel: { r: 0, g: 0, b: 0, a: 255 }, deltaE: 10 };
image.recolor({ mappings: [], material: white, materialRanges: [range] }, new Uint8Array(4));
// @ts-expect-error
image.recolor({ mappings: [], material: white, materialRanges: [{ pixel: white, deltaE: 1 }] }, new Uint8Array(4));

// An unprinted color in a config is either a stored color or the material's own color.
const unprinted: ConfigUnprinted[] = [
  { kind: 'material', deltaE: 10 },
  { kind: 'color', rgba: { r: 1, g: 2, b: 3, a: 255 }, deltaE: 2 },
];
void unprinted;
// @ts-expect-error
const noColor: ConfigUnprinted = { kind: 'color', deltaE: 2 };
void noColor;

export {};
