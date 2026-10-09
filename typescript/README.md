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
the Rust toolchain). While `npm run dev` runs, it rebuilds the package itself when the Rust it is
made from changes (`core`, `config`, `wasm`: sources and manifests; `wasm-watch.ts`), then reloads
the page; a failed build shows in the terminal and the browser's error overlay, and the page isn't
reloaded. `REKOLOR_WASM_WATCH=0 npm run dev` turns that off.

`npm ci` also installs the Git hooks (`../.husky/`, through the `prepare` script). Before each
commit they check what the commit touches: for `rust/`, `cargo fmt --check` and clippy for the
native and wasm32 builds; for `typescript/`, `npm run lint` and `npm run check`. They never change
or stage files. `git commit --no-verify` skips them; `HUSKY=0` disables them.

## Scripts

| Script                 | What                                                                                                                                                                                            |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `npm run wasm`         | Builds the WASM package.                                                                                                                                                                        |
| `npm run dev`          | Dev server.                                                                                                                                                                                     |
| `npm run build`        | Production build in `dist/` (base path `/`).                                                                                                                                                    |
| `npm run build:pages`  | Production build for GitHub Pages under `/reKolor/` (`REKOLOR_BASE` overrides the base).                                                                                                        |
| `npm run check`        | `svelte-check` (TypeScript and Svelte), warnings fail.                                                                                                                                          |
| `npm run lint`         | ESLint and Prettier.                                                                                                                                                                            |
| `npm test`             | Node tests (Vitest): the engine against the real WASM package, scheduling, zoom math, limits.                                                                                                   |
| `npm run test:browser` | Browser tests (Vitest browser mode, Playwright: Chromium, Firefox, WebKit): decoding vs Rust, the real worker, the app end to end. Needs `npx playwright install chromium firefox webkit` once. |
| `npm run smoke`        | Builds for `/reKolor/`, serves it and drives it in Chromium (worker, WASM and palette paths).                                                                                                   |

## How it works

```
main thread (Svelte)                      worker (src/lib/worker.ts)
  App.svelte ── EngineClient ──Comlink──►   Session (session.ts: generations, revisions, guards)
  ImageView × 2 (shared zoom/pan)             ├─ codec.ts: createImageBitmap + canvas (decode, PNG)
  PickList                                    └─ Engine (engine.ts) ── rekolor-wasm
```

- **Decoding** happens in the browser, inside the worker: EXIF orientation applied, the color
  profile converted to sRGB. The size limits (50 MB file, 16,384 px per side, 24 megapixels) are
  checked before the app makes its own pixel buffers. Known limitation: the width and height are
  only known once the browser has decoded the file, so a small, highly compressed file with huge
  dimensions can use a lot of memory (and crash the worker or the tab) before it is refused. Only
  the person who opened the file is affected.
- **Picking:** a click maps through the zoom/pan transform to a source pixel; Rust reads the stored
  pixel, composites it over the material and suggests the nearest ink. At 100 % and above the color
  on screen is sent too, and Rust warns if it differs from the stored pixel.
- **Moving a pick:** hold Shift and drag its circle (`src/lib/moves.ts`). The pick follows the
  pointer pixel by pixel, re-picked as if clicked there, with the nearest ink for each new color;
  within the same color a hand-chosen ink is kept. Pixels whose color another pick already has are
  skipped, so a drop there leaves the pick on the last free pixel it passed. Escape, or the browser
  taking the pointer away, cancels: the pick and its ink go back to how they were before the drag.
  Away from a circle, Shift changes nothing (drag pans, click picks). Picks imported from a config
  have no circle. Moving needs a mouse or pen and a keyboard; there is no touch or keyboard-only way.
  Circles are drawn on their own canvas, so the color read for the warning above is never a circle.
- **Material:** the garment or surface color, at the bottom right ("Material": a color input,
  "White", "Black" and "Reset", which goes back to none). None at first: the swatch and both frames
  show the checkerboard, and the engine composites over white. Once chosen, every engine call
  composites over it (picks, the preview, the color count), and the preview's frame shows it behind
  the result (as the frame's background, never in the image canvas); the Original keeps the
  checkerboard. Until the first pick nothing is printed: the preview shows only the material (or the
  checkerboard), and there is nothing to download. It applies live while the picker is open: inputs merge into the queued
  change while it hasn't started and nothing else was queued after it (`Serial.coalescing`), so a
  burst is one change and a change never passes another pick-list change made before it. A change
  re-matches the picks: one whose composited color changed gets the nearest ink again; the others
  keep theirs, hand-chosen or not. Picks that end up with the same color are all kept (the earlier
  one's ink prints that exact color). It isn't remembered across reloads.
- **Not printed:** colors the material already has don't need ink. "+" beside "Not printed", then a
  click on the original, adds that pixel's color with ΔE 10 (a slider, 0–40; 0–100 for an entry
  imported above 40); it is marked on the
  original with a square (picks have circles). Choosing a material adds the material's own color
  the same way: one entry that follows the material and keeps its ΔE (added again after × on the
  next choice). Every pixel within an entry's ΔE takes no ink and is transparent in the result, so
  the preview shows the material there and the downloaded PNG is transparent. Entries are checked
  before the picks, so a pick whose own color an entry covers says so in its row ("Its color is
  left unprinted"), by the same test recolor uses (`unprintedColors` in Rust). A new image clears the clicked entries; the material's stays. Exported configs
  carry the entries (`unprinted`); an import replaces them with the file's (imported ones have no
  square, as imported picks have no circle). At most 256 entries, the material's own included (the
  palette-config and WASM limit): a click past that is refused, and a material chosen when the
  list is full stays printed.
- **Layout:** toolbar (image, zoom, download), the two images, then at the bottom the picks on the
  left (as many columns as fit) and "Material" on the right (one column).
- **Live recolor:** at most one recolor runs and one waits (the newest picks); every result carries
  the image generation and pick revision, and only the current one is shown or downloaded. A failed
  open keeps the previous image (its generation changes only when an open succeeds).
- **Pick-list changes** (pick, move, ink change, ΔE, remove, clear, import, palette switch) run one
  at a time in the order they were made. At most 256 picks (the palette-config limit).
- **The ink palette** is the built-in `../palettes/pantone.json`, or a `*.json` file the user
  uploads ("Upload palette…" on the Picks line; at most 1 MB and 10,000 inks), in the same format
  (`../palettes/palette.schema.json`, checked against the reader in `src/lib/palette.test.ts`). The
  worker swaps the palette and keeps the open image; every pick then gets the new palette's nearest
  ink (its ΔE stays). The page (and the saved session) changes palette and inks in one step, after
  the new inks are known; if re-inking fails, the worker goes back to the previous palette.
  "Download palette" saves the current one, "Use Pantone" goes back. Config imports and exports name
  the current palette's inks. A restarted worker starts with Pantone, so the app hands it the
  uploaded palette again. The palette's name (an uploaded file's name) is private in LogRocket
  recordings.
- **A pick's ΔE** (its slider, 0–40; up to 100 from a file) is its capture radius: colors within it
  (CIEDE2000) of the pick's color print with its ink, before the nearest-ink rule (see "Capture
  radius" in `../rust/README.md`). A new pick starts at 0 (only its exact color); moving a pick
  keeps it; a slider drag is one coalesced change, like an unprinted color's.
- **Download** encodes the current result at full resolution (PNG, in the worker); if the image or
  picks change while it encodes, nothing is saved.
- **Palette configs:** import/export the picks as `*.palettes.toml`, the CLI's golden-set format,
  read and written by the shared Rust crate `rekolor-config`. Export always writes the material and
  the unprinted colors, and each pick's `delta_e` when it isn't 0; import sets them from the file
  (white and none if the file has no such lines) and keeps the file's picks as written. The file is
  the whole state: an import replaces the picks, the material and the unprinted colors. A white
  material counts as chosen only if the file lists the material's own color as unprinted.
- **Reloads keep the work** (`src/lib/persist.ts`): the picks, the material, the unprinted colors
  and an uploaded palette go to `sessionStorage` (per tab, about 5 MB) after every change; the open
  image goes to IndexedDB (no practical size limit), stored as bytes (WebKit can't store a `File` in
  a private window), under the tab's ID. Each open page holds a Web Lock named after its tab; a
  starting page deletes images whose tab holds no lock (closed tabs), after a 3-second grace for
  tabs that are reloading. A duplicated tab finds its copied ID's lock taken and takes a new ID with
  a copy of the image. At the first start the app restores, in order: the palette, the material, the
  image (only the one the state was saved with), then the picks and unprinted colors exactly as they
  were, unless the user has opened an image meanwhile (theirs wins). Nothing is saved before that,
  so a fresh start can't overwrite it; and if the saved image can't come back (missing, a different
  file, unreadable), the saved session stays until the next change, so a reload can try again. Image
  saves are numbered and only the newest writes, and the saved state names the image only once its
  bytes are stored, so a reload never pairs picks with another image. A save that doesn't fit leaves
  the previous one. Browser tests clear both stores first (`tests/browser/setup.ts`).
- **Errors** come back as values (`AppOutcome`); a crashed worker is restarted and the user is told.

## Session recording (LogRocket)

The published site records visits with [LogRocket](https://logrocket.com). `src/lib/logrocket.ts` is the only code that talks to it.

- **Where:** only on the published site. Everywhere else (the dev server, `vite preview`, the tests, CI's smoke test) LogRocket isn't even loaded:
  it is a separate file of the build, loaded with `import()` on the published site only, because
  loading it already contacts LogRocket's servers.
- **Visitors are told** on the privacy page, `public/privacy.html`, linked from the LogRocket logo
  at the right end of the header. It lists what is and isn't recorded; keep it in step with
  `src/lib/logrocket.ts`.
- **What is recorded:** the page's layout and text, clicks and other input (such as the material
  color and the inks), console output, uncaught errors, performance data, and the browser's details
  (type, system, screen, language, referring page), plus the events and error kinds below.
- **What is not recorded:** IP addresses (`shouldCaptureIP: false`), network requests
  (`network.isEnabled: false`), and every element marked `data-private`, which LogRocket never
  sends: the image views (`ImageView.svelte`: the user's image and its preview), and everything
  that can show a file name (the file details, the status and error lines, the palette dialog, the
  file inputs). A browser test checks that a file's name never appears outside such an element.
- **Release:** the commit CI built (`VITE_RELEASE`, set in `../.github/workflows/ci.yml`), so a
  recording says which code was running; `local` for builds made elsewhere.
- **Errors:** the kind of every error the app shows (`decodeFailed`, `invalidConfig`, …;
  `showError` in `App.svelte`), never the message, which can name the user's files. Errors in the
  engine's worker reach the page with their kind; LogRocket doesn't run in the worker.
- **No `identify`:** the app has no accounts, so recordings aren't linked to a person by name. They
  aren't anonymous: the browser's details are recorded.
- **Events** (`track`), searchable in LogRocket as custom events:

| Event              | When                                    | Properties                                 |
| ------------------ | --------------------------------------- | ------------------------------------------ |
| Image opened       | an image is open and its colors counted | `format` (\*), `width`, `height`, `colors` |
| Pick added         | a click on the original adds a pick     | `picks` (the count after)                  |
| Pick removed       | a pick's × button                       | `picks` (the count after)                  |
| Picks cleared      | "Clear all"                             | `removed`                                  |
| Picks imported     | a palette config is imported            | `size`, `picks`, `unprinted`               |
| Picks exported     | "Export picks"                          | `picks`, `unprinted`                       |
| PNG downloaded     | "Download PNG"                          | `picks`, `inks` (distinct inks)            |
| Palette uploaded   | "Upload palette…" (a valid palette)     | `inks`                                     |
| Palette downloaded | "Download palette"                      | `custom`, `inks`                           |
| Palette reset      | "Use Pantone"                           | none                                       |

(\*) A name from a fixed list (`imageFormat` in `src/lib/logrocket.ts`), found from the file's type
or, without one, its extension; anything else is `other`, so no part of a file name is sent.

Tests: `src/lib/logrocket.test.ts` (with LogRocket replaced by a fake); in the browser,
`tests/browser/app.test.ts` (the private parts and the privacy link) and
`tests/browser/events.test.ts` (what "Image opened" carries).

## Browser differences found by the cross-decoder test

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

The Rust, WebAssembly, TypeScript and Svelte logos in the header (`src/components/BuiltWith.svelte`)
are drawn from [Simple Icons](https://simpleicons.org) 16.32.0 (CC0). The Rust logo is the Rust
Foundation's, licensed [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) (see the
[Rust media guide](https://www.rust-lang.org/policies/media-guide)). The logos are trademarks of
their owners and link to their projects' sites.

The LogRocket logo at the right end of the header (`src/components/LogRocketLink.svelte`) is
LogRocket's rocket mark, taken from the logotype in the header of
[logrocket.com](https://logrocket.com); it is a trademark of LogRocket, Inc., used to name the
service the site records visits with, and links to the privacy page.
