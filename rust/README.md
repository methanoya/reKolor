# reKolor, Rust part

A self-contained Cargo workspace: the color engine, its browser interface, native file
I/O and a command-line tool. It builds and tests on its own; the web app (`../typescript/`) consumes only
the WASM package it produces.

| Crate | What it is | Must not depend on |
|---|---|---|
| `crates/core` (`rekolor-core`) | Pure color logic on raw RGBA8 buffers: `recolor`, `analyze`, `Palette` (suggest, nearest), `pick` by coordinate, typed errors. No I/O, no printing. | wasm-bindgen, tsify, `image` |
| `crates/config` (`rekolor-config`) | Palette config files (`*.palettes.toml`): parse, validate (sizes, distinct inks, limits), write, and resolve picks against a `Palette` (compositing in Rust). Shared by the CLI and the web app. No file I/O. | `rekolor-io`, wasm-bindgen |
| `crates/wasm` (`rekolor-wasm`) | Thin browser interface over `core` and `config` (wasm-bindgen + tsify). Classes `SourceImage` and `Palette`, config functions; results as `Outcome<T>` values. | `rekolor-io` (it would pull `image` into the WASM build) |
| `crates/io` (`rekolor-io`) | Native decoding of every readable `image` format to RGBA8 (EXIF orientation applied), PNG encoding, and the decoder-discrepancy warnings as data. | — |
| `crates/cli` (`rekolor-cli`, binary `rekolor`) | Command-line tool on top of `io`, `config` and `core`; also a small library (config file names and reading, palette loading, config generation, discovery) used by the golden test. | — |

Shared test data lives in `testdata/` (see [Baseline snapshot](#baseline-snapshot)); the golden set lives in
`../samples/` (see [Golden set](#golden-set)); the palette is `../palettes/pantone.json`.

## Requirements

- **rustup.** `rust-toolchain.toml` selects stable Rust with the `wasm32-unknown-unknown` target,
  clippy and rustfmt; rustup installs them on first use. Run cargo from inside `rust/`, where
  rustup finds that file. (Homebrew's `rust` package can't add the WASM target.)
- **wasm-pack** 0.15 (`brew install wasm-pack` or `cargo install wasm-pack --locked`).
- **Node 24**, for the WASM tests and the TypeScript contract test.

## Checks

From `rust/`:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p rekolor-wasm --target wasm32-unknown-unknown --all-targets -- -D warnings

cargo test --workspace                                          # native: unit, baseline, io, CLI end-to-end
wasm-pack test --node crates/wasm                               # inside WASM: baseline, ΔE, boundary, errors (~15 s)
cargo test --release -p rekolor-cli --test golden -- --ignored  # golden set (opt-in, ~5 s)

# The shipped code is a release build, so the ΔE fingerprint runs in release too:
cargo test --workspace --release
wasm-pack test --node --release crates/wasm

# `palette` must resolve only `libm`, never `std` (see "Color math"):
cargo tree -e features -i palette_math
cargo tree -e features -i palette_math --target wasm32-unknown-unknown
```

TypeScript contract test (types and runtime behavior of the built package, as an app sees it):

```sh
wasm-pack build crates/wasm --release --target web   # the package must exist before `npm ci`
cd crates/wasm/ts-test
npm ci
npm test                                             # tsc --noEmit, then vitest
```

| Suite | Where | What it proves |
|---|---|---|
| core unit tests | `crates/core/src/*` | validation, errors, compositing, matching, ties, palette rule, pick, ΔE against the Sharma reference data |
| baseline | `crates/core/tests/baseline.rs` | `core` matches the baseline snapshot pixel for pixel |
| suggestions | `crates/core/tests/suggestions.rs` | Pantone suggestions match the snapshot exactly (5,096 distinct colors) |
| fingerprint | `crates/core/tests/fingerprint.rs` | ΔE reproduces the recorded fingerprint bit for bit |
| WASM | `crates/wasm/tests/node.rs` | the baseline inside WASM, the same ΔE fingerprint, Sharma data and suggestion snapshot (native = WASM, bit for bit), exact ties, malformed input → error values, lengths, factories, pick |
| contract | `crates/wasm/ts-test/` | the generated `.d.ts` (camelCase, `Outcome` narrowing, typed arrays) and runtime behavior from TypeScript |
| io | `crates/io/tests/io.rs` | lossless PNG round trip, 16-bit/grayscale, ICC and EXIF warnings, typed errors, all samples decode |
| decoder fixtures | `crates/io/tests/decoders.rs` | the cross-decoder fixtures match a fresh `rekolor-io` decode |
| config | `crates/config/src/lib.rs` | config format, round trip, escaping, limits, rules, resolving picks |
| CLI | `crates/cli/src/*`, `crates/cli/tests/cli.rs` | configs, generator rules, discovery; the real binary end to end |
| golden | `crates/cli/tests/golden.rs` | every sample matches its reviewed outputs (opt-in) |

## WASM package

```sh
wasm-pack build crates/wasm --release --target web   # → crates/wasm/pkg (ignores itself via pkg/.gitignore)
```

The package's `.d.ts` carries the whole contract. Usage from TypeScript (in the app this belongs in
a Web Worker):

```ts
import init, { Palette, SourceImage, composite, parseConfig, serializeConfig, unprintedColors } from 'rekolor-wasm';

await init();

// palettes/pantone.json: Object.entries keeps the file order, which decides ties.
const json: Record<string, { rgb: [number, number, number] }> = await (await fetch(paletteUrl)).json();
const palette = Palette.create({
  entries: Object.entries(json).map(([name, { rgb: [r, g, b] }]) => ({ name, rgb: { r, g, b } })),
});
if (palette.status === 'error') throw new Error(palette.error.message);

const created = SourceImage.create(new Uint8Array(imageData.data.buffer), imageData.width, imageData.height);
if (created.status === 'error') throw new Error(created.error.message);
const image = created.value;

// The material (garment or substrate) every call composites over; white = the behavior before it.
const material = { r: 255, g: 255, b: 255 };

const pick = image.pick(x, y, seenColor, material, palette.value); // x, y in image pixels
if (pick.status === 'ok' && pick.value.mismatch) console.warn('coordinate mapping?', pick.value.mismatch);

const out = new ImageData(image.width, image.height);
const result = image.recolor({ mappings, material }, new Uint8Array(out.data.buffer)); // writes into `out`
// Colors left unprinted (optional): no ink, transparent in the output, so the material shows.
const ranges = [{ pixel: { ...material, a: 255 }, deltaE: 10 }];  // the material's own color
image.recolor({ mappings, material, materialRanges: ranges }, new Uint8Array(out.data.buffer));
const matching = composite({ r: 203, g: 0, b: 0, a: 100 }, material); // a pixel as recolor sees it
// Which colors (as recolor matches them) the ranges leave unprinted, by recolor's own test:
const flags = unprintedColors({ colors: [matching], material, materialRanges: ranges }); // Outcome<{ unprinted: boolean[] }>

image.free(); // or `using` / Symbol.dispose

// Palette configs (*.palettes.toml, the golden-set format), read and written by `rekolor-config`:
const parsed = parseConfig(tomlText);    // Outcome<{ material, unprinted, sections: [{ size, picks }] }>
const picks = palette.value.resolveSection(section, parsed.value.material); // matching colors from Rust
const text = serializeConfig({ imageName: 'tiger.png', material, picks: [{ rgba, ink: 'Pantone 1595' }],
  unprinted: [{ kind: 'material', deltaE: 10 }, { kind: 'color', rgba, deltaE: 12.5 }] });
```

The material is a required argument everywhere it is used (`pick`, `recolor`, `colorCount`,
`analyze`, `resolveSection`, `serializeConfig`): there is no hidden white default at the boundary.
`SourceImage.colorCount(material)` is the bounded color count (2 MiB, whatever the image); `recolor`
accepts at most 256 mappings (`tooManyMappings`) and 256 material ranges (`tooManyRanges`), the
palette-config limits, and a range's `deltaE` from 0 to 100 (`invalidInput` otherwise);
`unprintedColors` checks its ranges the same way.

The WASM file is about 360 KB (150 KB gzipped); the TOML parser for `*.palettes.toml` is a large
part of it.

Every call that can fail on input returns `{ status: 'ok', value } | { status: 'error', error: { kind, message } }`.
Exceptions only signal programming mistakes or Rust panics, which `console_error_panic_hook`
prints to the console.

## CLI

```sh
cargo build --release -p rekolor-cli                 # → target/release/rekolor
```

```sh
rekolor recolor <input> -o <out.png> --config <name.palettes.toml> --size <N> [--material R,G,B]
rekolor recolor <input> -o <out.png> --pick "255,184,0=Pantone 1235" [--pick …] [--material R,G,B]
                [--unprinted R,G,B[,A]=ΔE | material=ΔE …]
rekolor analyze <input> [--material R,G,B]
rekolor palettes generate <dir> [--force] [--material R,G,B]   # <name>.palettes.toml next to every image
rekolor golden update <dir>                 # <name>-out-<size>.png from each config
```

- `--material R,G,B`: the material color the image is composited over (the garment or substrate).
  Default white; with `--config`, the config's `material`, which an explicit `--material` overrides
  (the config's inks stay as written). `palettes generate` matches on it and writes it into each
  config, with the material's own color as an unprinted entry (ΔE 10, as the app does when a material
  is chosen); `golden update` uses each config's own.
- `--unprinted`: a color left unprinted, transparent in the output: `R,G,B[,A]=ΔE`, or
  `material=ΔE` for the material's own color. Repeatable; only with `--pick` (with `--config`, the
  config's are used). ΔE from 0 to 100.

- `--palette <path>` on every command; by default `palettes/pantone.json` in the current directory
  or the nearest parent that has it.
- Inputs: every format `rekolor-io` reads. Decoder warnings (ICC profile ignored, EXIF orientation
  applied, semi-transparent pixels) go to stderr; errors exit non-zero.
- `log` traces go to stderr at `warn` level by default; set `RUST_LOG=debug` for more.

## Golden set

Every image in `../samples/**` has, next to it:

- `<name>.palettes.toml`: picks for 3, 7 and 16 inks (`{ rgba = [r, g, b, a], ink = "<name>" }`).
  "Size N" means up to N distinct inks. A top-level `material = [r, g, b]` (before the first
  `[[palette]]`) is the material the picks are composited over; it is always written, and a file
  without it is read as white (the current golden configs have none). A top-level `unprinted` list
  holds the colors left unprinted, for every size: `{ rgba = [r, g, b, a], delta_e = 10 }` or
  `{ material = true, delta_e = 10 }` (the material's own color, whatever it is); always written
  (`unprinted = []` for none), none when absent, ΔE 0–100, at most 256, one material entry. The
  first version was generated by `rekolor palettes generate` (predominant, mutually most different
  colors: see `crates/cli/src/generate.rs`); since then the file is the source of truth and may be
  hand-edited.
- `<name>-out-{3,7,16}.png`: the reviewed outputs.

The golden test only reads and compares decoded pixels exactly. On a mismatch it reports the number
of differing pixels with the first coordinates, and writes a diff image (magenta on a faded copy of
the expected output) to `$TMPDIR/rekolor-golden-diff/`.

After an intentional change (a behavior change, an edited config):

```sh
target/release/rekolor golden update ../samples   # rewrite the outputs
git diff --stat ../samples                        # review the changed images, then commit
```

`palettes generate` keeps existing configs; `--force` regenerates them and overwrites hand edits.

## Baseline snapshot

`testdata/baseline/` holds the **current approved behavior** of `core` on generated inputs. Tests only
read it; after an intentional change, regenerate it and review the diff:

```sh
cargo run --release -p rekolor-core --example update_baseline                          # rewrite what changed
cargo run --release -p rekolor-core --example update_baseline -- --colors-from-current # same, keeping the suggestion colors
git diff --stat testdata/baseline                                            # review, then commit
```

The command reports which files changed ("baseline unchanged" otherwise). Recolor PNGs are rewritten
only when their decoded pixels differ.

| Path | What |
|---|---|
| `testdata/generator.rs` | Deterministic inputs, plain Rust with no dependencies, included as a module wherever needed. Fixtures: `alpha_ramp` (every alpha 0–255 on 8 colors), `edges` (alpha- and color-anti-aliased edges on a transparent background), `picks` (exact pick colors and their ±1 neighbors), `gradient` (many unique colors), `noise` (random RGBA), `transparent` (fully transparent pixels with hidden RGB). Mapping sets: `none`, `one`, `calendar3`, `screenshot8`, `swapped4`. Suggestion colors: a 16-level grid plus 1,000 distinct off-grid colors (5,096 distinct). |
| `testdata/baseline/recolor/<fixture>__<mapping set>.png` | `recolor` for every fixture × mapping set. Compared as **decoded pixels**. |
| `testdata/baseline/image-info.tsv` | `analyze` for every fixture. |
| `testdata/baseline/pantone-suggestions.tsv` | `Palette::suggest` for the suggestion colors, with ΔE. Compared exactly. |
| `testdata/baseline/delta-e-fingerprint.tsv` | ΔE fingerprint: a hash over the `f32` bits of ΔE for 100,000 generated color pairs, plus exact values for a few pairs. Native and WASM tests must reproduce it bit for bit. |
| `testdata/ciede2000-sharma.tsv` | CIEDE2000 reference data (not generated; see "Color math"). |
| `testdata/decoders/` | Cross-decoder fixtures: encoded images (opaque, EXIF PNG and JPEG, alpha ramp, ICC profile) and how `rekolor-io` decodes them (`<name>.rgba`, `references.tsv`). The web app's browser tests compare the browser's decoding with these. Regenerate with `cargo run --release -p rekolor-io --example update_decoder_fixtures`. |

## Color math

Lab conversion and CIEDE2000 come from the [`palette`](https://crates.io/crates/palette) crate
(0.7.7), in `f32`. `core` exposes `Lab` (`palette::Lab<D65, f32>`), `lab(Rgb8)` (from encoded sRGB) and
`delta_e_2000_lab(Lab, Lab)`, beside `delta_e_2000(Rgb8, Rgb8)`; the `palette` crate itself isn't
re-exported.

- **Same results on every target.** `palette` is built with `default-features = false, features =
  ["libm"]`, so its float functions (`powf`, `atan2`, `sin`, `cos`, `exp`, …) are the pure-Rust `libm`
  crate everywhere. With the default `std` feature they'd come from the platform's math library natively
  (Apple's libm on macOS) but from Rust's bundled musl port in WASM, and ΔE differed by about 1e-4 ΔE
  between the CLI and the browser. **Tested scope:** the fingerprint covers ΔE between Lab values
  converted from 8-bit sRGB colors (`delta_e_2000`, everything `recolor` and the palette use).
  `delta_e_2000_lab` on other Lab values (e.g. out of the sRGB gamut) uses the same code but isn't
  checked bit for bit; the Sharma pairs check it within `1e-4` on each target. **Don't enable
  `palette/std` anywhere in the workspace:** Cargo
  features are additive, so one dependency doing it would bring the difference back. The fingerprint
  tests would catch it; the `cargo tree` checks above show the resolved features.
- **Cost:** natively, `libm` is about 2.5× slower than `std` for ΔE. In WASM there's no extra cost:
  `std` already uses the musl port there.
- **Speed and memory:** `recolor` remembers each composited color's answer in a fixed 32 MiB table
  for images over 65,536 pixels, so repeated colors skip the CIEDE2000 work (identical output, checked
  by the goldens, the baseline and an equivalence test).
  `color_count` counts distinct colors with a fixed 2 MiB bit table; `analyze` also counts RGBA
  values, which needs memory per distinct value.
- **Reference data:** `testdata/ciede2000-sharma.tsv` holds the 34 test pairs from G. Sharma, W. Wu and
  E. N. Dalal, "The CIEDE2000 color-difference formula: implementation notes, supplementary test data,
  and mathematical observations", *Color Research & Application* 30(1), 2005. Retrieved 2026-10-07 from
  <https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/ciede2000testdata.txt>; the
  values are unchanged, tab-separated as published, with a `#` header line added (34 rows). Native and
  WASM tests check every pair within `1e-4` (the reference has 4 decimals).

## Behavior

- **Matching:** each pixel is composited over the material color (white by default) and takes the
  ink nearest to it by CIEDE2000; a pixel whose composited color equals a mapping's source takes that
  mapping's ink. The output is opaque, except for colors left unprinted.
- **Material color:** per channel `(a·c + (255 − a)·m) / 255`, truncating, on the stored sRGB
  values. `pick`, `recolor`, `analyze`/`color_count`, config resolving and the generator take the
  material.
- **Colors left unprinted:** `recolor_with_ranges` takes material ranges, each a stored pixel and a
  ΔE. A pixel whose composited color is within a range's ΔE (CIEDE2000) of the range's pixel
  composited over the material takes no ink and is **transparent** (`[0, 0, 0, 0]`) in the output, so
  the material shows through; `RecolorStats::unprinted` counts them. Ranges are checked before the
  mappings, so they win over an exact pick. With no ranges the output is exactly `recolor`'s.
  Configs carry them as `unprinted` (see "Golden set"); the CLI's `recolor` and `golden update` use
  them.
- **Suggestions:** `Palette::suggest` is the plain nearest entry (CIEDE2000), real ink or not
  ("Pure White/Black (non-palette)" included). The config generator skips any candidate whose ink is
  already taken, so one changed suggestion can move later picks.
- **Zero and one mapping:** with no mappings, `recolor` returns the composited copy (the image as it
  looks on the material); with one mapping, every pixel takes that ink. Tested in `core`, WASM and
  the TS contract.
- **Color counts:** `analyze` reports `colors`, the number of distinct colors after compositing over
  the material (what `recolor` matches), and `rgbaColors`, the distinct RGBA values as stored.
- **Ties:** palette file order decides ties between entries with the same color (e.g. Pantone 303
  before 547); `palettes/pantone.json` is a JSON object, and every reader keeps its key order
  (`serde_json` with `preserve_order`, `Object.entries`). When a pixel is equally near to two
  mappings (or matches two exactly), the earlier mapping wins.
- **Native vs WASM:** ΔE is bit-identical in native and WASM builds, so ties and near-ties resolve
  the same way in the browser as in the CLI and goldens (see "Color math").

Each behavior change updates the baseline snapshot (and, if pixels move, the goldens) in the same
commit, so its effect is a reviewed diff.
