# reKolor web app

Preview a picture printed with a limited ink palette: open an image, click the colors to keep, and
each one is replaced by its nearest Pantone ink (CIEDE2000). The recolored preview updates live.

Svelte 5 + TypeScript 6 + Vite 8. All color logic runs in Rust: the `rekolor-wasm` package
(`../rust/crates/wasm`) in a Web Worker. TypeScript only handles the browser: files, canvas, UI.

## From a clean clone

The WASM package (`../rust/crates/wasm/pkg`) is generated and not committed, and `package.json`
depends on it as a local package, so build it **before** `npm ci`:

```sh
cd typescript
npm run wasm        # wasm-pack build ../rust/crates/wasm --release --target web
npm ci
npm run dev         # http://localhost:5173
```

`npm run wasm` works before `npm ci` because it only calls `wasm-pack` (see `../rust/README.md` for
the Rust toolchain). Rebuild it after changing Rust code; the dev server picks it up.

## Scripts

| Script                 | What                                                                                                                                                                                                 |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `npm run wasm`         | Builds the WASM package.                                                                                                                                                                             |
| `npm run dev`          | Dev server.                                                                                                                                                                                          |
| `npm run build`        | Production build in `dist/` (base path `/`).                                                                                                                                                         |
| `npm run build:pages`  | Production build for GitHub Pages under `/reKolor/` (`REKOLOR_BASE` overrides the base).                                                                                                             |
| `npm run check`        | `svelte-check` (TypeScript and Svelte), warnings fail.                                                                                                                                               |
| `npm run lint`         | ESLint and Prettier.                                                                                                                                                                                 |
| `npm test`             | Node tests (Vitest): the engine against the real WASM package, scheduling, zoom math, limits.                                                                                                        |
| `npm run test:browser` | Browser tests (Vitest browser mode, Playwright: Chromium, Firefox, WebKit): decoding vs Rust (M2), the real worker, the app end to end. Needs `npx playwright install chromium firefox webkit` once. |
| `npm run smoke`        | Builds for `/reKolor/`, serves it and drives it in Chromium (worker, WASM and palette paths).                                                                                                        |

## How it works

```
main thread (Svelte)                      worker (src/lib/worker.ts)
  App.svelte ── EngineClient ──Comlink──►   Session (session.ts: generations, revisions, guards)
  ImageView × 2 (shared zoom/pan)             ├─ codec.ts: createImageBitmap + canvas (decode, PNG)
  PickList                                    └─ Engine (engine.ts) ── rekolor-wasm
```

- **Decoding** happens in the browser (R5), inside the worker: EXIF orientation applied, the color
  profile converted to sRGB. The size limits (50 MB file, 16,384 px per side, 24 megapixels) are
  checked before any pixel buffer is made.
- **Picking:** a click maps through the zoom/pan transform to a source pixel; Rust reads the stored
  pixel, composites it over the material and suggests the nearest ink (R10). At 100 % and above the color
  on screen is sent too, and Rust warns if it differs from the stored pixel.
- **Moving a pick:** hold Shift and drag its circle (`src/lib/moves.ts`). The pick follows the
  pointer pixel by pixel, re-picked as if clicked there, with the nearest ink for each new color;
  within the same color a hand-chosen ink is kept. Pixels whose color another pick already has are
  skipped, so a drop there leaves the pick on the last free pixel it passed. Escape, or the browser
  taking the pointer away, cancels: the pick and its ink go back to how they were before the drag.
  Away from a circle, Shift changes nothing (drag pans, click picks). Picks imported from a config
  have no circle. Moving needs a mouse or pen and a keyboard; there is no touch or keyboard-only way.
  Circles are drawn on their own canvas, so the color read for the warning above is never a circle.
- **Material:** the garment or surface color (toolbar: a color input and "White"; white by
  default). Every engine call composites over it: picks, the preview, the color count. The Original
  panel shows it behind the image (as the frame's background, never in the image canvas). It applies
  live while the picker is open: inputs merge into the queued change while it hasn't started and
  nothing else was queued after it (`Serial.coalescing`), so a burst is one change and a change never
  passes another pick-list change made before it. A change re-matches the picks: one whose composited color changed gets the
  nearest ink again; the others keep theirs, hand-chosen or not. Picks that end up with the same color
  are all kept (the earlier one's ink prints that exact color). It isn't remembered across reloads.
- **Live recolor:** at most one recolor runs and one waits (the newest picks); every result carries
  the image generation and pick revision, and only the current one is shown or downloaded. A failed
  open keeps the previous image (its generation changes only when an open succeeds).
- **Pick-list changes** (pick, move, ink change, remove, clear, import) run one at a time in the order they
  were made. At most 256 picks (the palette-config limit).
- **Download** encodes the current result at full resolution (PNG, in the worker); if the image or
  picks change while it encodes, nothing is saved.
- **Palette configs:** import/export the picks as `*.palettes.toml`, the CLI's golden-set format,
  read and written by the shared Rust crate `rekolor-config`. Export always writes the material;
  import sets it from the file (white if the file has none) and keeps the file's picks as written.
- **Errors** come back as values (`AppOutcome`); a crashed worker is restarted and the user is told.

## Browser differences found by the cross-decoder test (M2)

`tests/browser/decoders.test.ts` compares the browser's decoding with `rekolor-io` on the fixtures in
`../rust/testdata/decoders/` (regenerate with
`cargo run --release -p rekolor-io --example update_decoder_fixtures`):

| Fixture                        | Chromium                    | Firefox         | WebKit    |
| ------------------------------ | --------------------------- | --------------- | --------- |
| Opaque sRGB PNG                | identical                   | identical       | identical |
| EXIF orientation (PNG)         | identical                   | identical       | identical |
| EXIF orientation (JPEG)        | same rotation, close colors | same            | same      |
| Alpha ramp (colors over white) | within ±1                   | within ±1       | within ±1 |
| ICC profile                    | applied                     | **not applied** | applied   |

- WebKit mishandles bitmaps decoded with `premultiplyAlpha: 'none'` when drawn onto a canvas
  (semi-transparent colors came back up to 64 off over white), so the codec uses the default.
- Firefox returns the stored values for an ICC-tagged PNG in this path, while Chromium and WebKit
  apply the profile, so such images can pick slightly different colors in Firefox. `rekolor-io`
  (CLI, goldens) keeps the stored values and warns.

## Credits

The favicon (from the 2023 app) is a rainbow icon by Freepik from Flaticon:
[Rainbow icons created by Freepik - Flaticon](https://www.flaticon.com/free-icons/rainbow)
(the attribution is also in `public/favicon.html`).
