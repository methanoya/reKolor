# reKolor, Rust part

A self-contained Cargo workspace (R1, R2): the color engine, its browser interface, native file
I/O and a command-line tool. It builds and tests on its own; the web app (`../typescript/`) consumes only
the WASM package it produces.

| Crate | What it is | Must not depend on |
|---|---|---|
| `crates/core` (`rekolor-core`) | Pure color logic on raw RGBA8 buffers: `recolor`, `analyze`, `Palette` (suggest, nearest), `pick` by coordinate, typed errors. No I/O, no printing. | wasm-bindgen, tsify, `image` |
| `crates/config` (`rekolor-config`) | Palette config files (`*.palettes.toml`): parse, validate (sizes, distinct inks, limits), write, and resolve picks against a `Palette` (compositing in Rust). Shared by the CLI and the web app (W10 v). No file I/O. | `rekolor-io`, wasm-bindgen |
| `crates/wasm` (`rekolor-wasm`) | Thin browser interface over `core` and `config` (wasm-bindgen + tsify). Classes `SourceImage` and `Palette`, config functions; results as `Outcome<T>` values. | `rekolor-io` (it would pull `image` into the WASM build) |
| `crates/io` (`rekolor-io`) | Native decoding of every readable `image` format to RGBA8 (EXIF orientation applied), PNG encoding, and the decoder-discrepancy warnings (M1) as data. | — |
| `crates/cli` (`rekolor-cli`, binary `rekolor`) | Command-line tool on top of `io`, `config` and `core`; also a small library (config file names and reading, palette loading, config generation, discovery) used by the golden test. | — |

Shared test data lives in `testdata/` (see [Baseline snapshot](#baseline-snapshot-r9-p1)); the golden set lives in
`../samples/` (see [Golden set](#golden-set-x1)); the palette is `../palettes/pantone.json`.

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
cargo test --release -p rekolor-cli --test golden -- --ignored  # golden set (opt-in, ~40 s)

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
| decoder fixtures | `crates/io/tests/decoders.rs` | the cross-decoder fixtures (M2) match a fresh `rekolor-io` decode |
| config | `crates/config/src/lib.rs` | config format, round trip, escaping, limits, rules, resolving picks |
| CLI | `crates/cli/src/*`, `crates/cli/tests/cli.rs` | configs, generator rules, discovery; the real binary end to end |
| golden | `crates/cli/tests/golden.rs` | every sample matches its reviewed outputs (opt-in) |

## WASM package

```sh
wasm-pack build crates/wasm --release --target web   # → crates/wasm/pkg (ignores itself via pkg/.gitignore)
```

The package's `.d.ts` carries the whole contract. Usage from TypeScript (in the app this belongs in
a Web Worker, T7):

```ts
import init, { Palette, SourceImage, parseConfig, serializeConfig } from 'rekolor-wasm';

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

const pick = image.pick(x, y, seenColor, palette.value);          // x, y in image pixels
if (pick.status === 'ok' && pick.value.mismatch) console.warn('coordinate mapping?', pick.value.mismatch);

const out = new ImageData(image.width, image.height);
const result = image.recolor({ mappings }, new Uint8Array(out.data.buffer)); // writes into `out`

image.free(); // or `using` / Symbol.dispose

// Palette configs (*.palettes.toml, the golden-set format), read and written by `rekolor-config`:
const parsed = parseConfig(tomlText);                         // Outcome<{ sections: [{ size, picks }] }>
const picks = palette.value.resolveSection(section);          // inks must exist; matching colors from Rust
const text = serializeConfig({ imageName: 'tiger.png', picks: [{ rgba, ink: 'Pantone 1595' }] });
```

The `palettes.toml` support adds the TOML parser to the WASM file: about 306 KB (131 KB gzipped),
up from 126 KB (56 KB gzipped) before.

Every call that can fail on input returns `{ status: 'ok', value } | { status: 'error', error: { kind, message } }`
(T5). Exceptions only signal programming mistakes or Rust panics, which `console_error_panic_hook`
prints to the console.

## CLI

```sh
cargo build --release -p rekolor-cli                 # → target/release/rekolor
```

```sh
rekolor recolor <input> -o <out.png> --config <name.palettes.toml> --size <N>
rekolor recolor <input> -o <out.png> --pick "255,184,0=Pantone 1235" [--pick …]
rekolor analyze <input>
rekolor palettes generate <dir> [--force]   # <name>.palettes.toml next to every image
rekolor golden update <dir>                 # <name>-out-<size>.png from each config
```

- `--palette <path>` on every command; by default `palettes/pantone.json` in the current directory
  or the nearest parent that has it.
- Inputs: every format `rekolor-io` reads. Decoder warnings (ICC profile ignored, EXIF orientation
  applied, semi-transparent pixels) go to stderr; errors exit non-zero.
- `log` traces go to stderr at `warn` level by default; set `RUST_LOG=debug` for more.

## Golden set (X1)

Every image in `../samples/**` has, next to it:

- `<name>.palettes.toml`: picks for 3, 7 and 16 inks (`{ rgba = [r, g, b, a], ink = "<name>" }`).
  "Size N" means up to N distinct inks. The first version was generated by
  `rekolor palettes generate` (predominant, mutually most different colors: see
  `crates/cli/src/generate.rs`); since then the file is the source of truth and may be hand-edited.
- `<name>-out-{3,7,16}.png`: the reviewed outputs.

The golden test only reads and compares decoded pixels exactly. On a mismatch it reports the number
of differing pixels with the first coordinates, and writes a diff image (magenta on a faded copy of
the expected output) to `$TMPDIR/rekolor-golden-diff/`.

After an intentional change (a behavior decision, an edited config):

```sh
target/release/rekolor golden update ../samples   # rewrite the outputs
git diff --stat ../samples                        # review the changed images, then commit
```

`palettes generate` keeps existing configs; `--force` regenerates them and overwrites hand edits.

## Baseline snapshot (R9, P1)

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
| `testdata/baseline/recolor/<fixture>__<mapping set>.png` | `recolor` for every fixture × mapping set. Compared as **decoded pixels** (X2). |
| `testdata/baseline/image-info.tsv` | `analyze` for every fixture. |
| `testdata/baseline/pantone-suggestions.tsv` | `Palette::suggest` for the suggestion colors, with ΔE. Compared exactly. |
| `testdata/baseline/delta-e-fingerprint.tsv` | ΔE fingerprint (R11): a hash over the `f32` bits of ΔE for 100,000 generated color pairs, plus exact values for a few pairs. Native and WASM tests must reproduce it bit for bit. |
| `testdata/ciede2000-sharma.tsv` | CIEDE2000 reference data (not generated; see "Color math"). |
| `testdata/decoders/` | Cross-decoder fixtures (M2): encoded images (opaque, EXIF PNG and JPEG, alpha ramp, ICC profile) and how `rekolor-io` decodes them (`<name>.rgba`, `references.tsv`). The web app's browser tests compare the browser's decoding with these. Regenerate with `cargo run --release -p rekolor-io --example update_decoder_fixtures`. |

### History

- The baseline was first recorded from the existing crate after the dependency upgrade (R12), at commit
  `c0d094c` ("Record the baseline"), which contains the old sources and the recorder
  (`rust/src/`, `rust/examples/record_baseline.rs`). `crates/core` reproduced those 30 recolor outputs
  and the image info exactly, natively and in WASM.
- Until behavior step 1, `pantone-suggestions.tsv` held the **2023 JavaScript** suggestions
  (`typescript/src/palette.ts` with `color-diff`, recorded by `testdata/record_pantone_suggestions.cjs`,
  since removed; both are in git history).
  The Rust suggestions matched all 5,096 by name; ΔE differed by at most 0.005 (f32 vs f64). That
  file is in git history; the snapshot now holds the Rust suggestions, for a new color set (the old one
  repeated after 256 of its 1,000 pseudo-random colors).

## Color math (R11)

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
- **Cost:** natively, `libm` is about 2.5× slower than `std` for ΔE (the golden test went from ~15 s to
  ~40 s). In WASM there's no extra cost: `std` already uses the musl port there.
- **Reference data:** `testdata/ciede2000-sharma.tsv` holds the 34 test pairs from G. Sharma, W. Wu and
  E. N. Dalal, "The CIEDE2000 color-difference formula: implementation notes, supplementary test data,
  and mathematical observations", *Color Research & Application* 30(1), 2005. Retrieved 2026-10-07 from
  <https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/ciede2000testdata.txt>; the
  values are unchanged, tab-separated as published, with a `#` header line added (34 rows). Native and
  WASM tests check every pair within `1e-4` (the reference has 4 decimals). The previous implementation
  (`lab` + `deltae`) passed them too; its ΔE differed from `palette`'s by up to 0.001 because of the
  sRGB→Lab conversion (up to ~0.005 per L/a/b channel).

## Behavior

The engine reproduces the existing recolor behavior (R9): nearest-ink matching, compositing over
white, opaque output, earlier mapping wins ties. The behavior decisions (`.agents/behavior-issues/`,
local) kept those; changes so far:

- **I5:** `Palette::suggest` is the plain nearest entry (CIEDE2000, ties to the earlier entry). The
  2023 preference for real inks over "Pure White/Black (non-palette)" was dropped; on the snapshot's
  5,096 colors this changed 6 suggestions, all near-black or near-white. The recolor algorithm didn't
  change, so the 30 generated recolor baseline PNGs stayed the same. The owner then chose to
  regenerate the golden configs, which changed two configs and four outputs:
  - `02-scarlet-macaw`: the background pick `[244,239,235]` goes from Pantone 427 to Pure White at all
    three sizes. At size 16 the freed Pantone 427 then goes to `[225,219,207]`, replacing the
    `[171,59,47]` / Pantone 1805 pick.
  - `06-neon-street` size 16: the `[255,255,239]` candidate now suggests Pure White, which is already
    taken, so the generator skips it and `[76,41,48]` / Pantone 5185 takes the slot.

  The generator skips any candidate whose ink is already taken, so one renamed suggestion can move
  later picks.

- **I8 (contract, no change):** with no mappings, `recolor` returns the composited copy (the image as
  it looks on white); with one mapping, every pixel takes that ink. Tested in `core`, WASM and the TS
  contract.

- **I4:** `analyze` reports `colors`, the number of distinct colors after compositing over white
  (what `recolor` matches), instead of the old RGB count that ignored alpha; `rgbaColors` stays. In the
  snapshot this changed the 4 fixtures with transparency (e.g. `transparent`: 256 → 1).

- **I7 (documented, no change):** palette file order decides ties between entries with the same color
  (e.g. Pantone 303 before 547). `palettes/pantone.json` stays a JSON object; every reader keeps its
  key order (`serde_json` with `preserve_order`, `Object.entries`).
- **I9 (documented, no change):** when a pixel is equally near to two mappings (or matches two exactly),
  the earlier mapping wins.

- **R11 (`palette` crate, see "Color math"):** no suggestion name changed; ΔE values in the snapshot
  moved by at most 0.001. 5 pixels changed in 3 recolor baseline PNGs. With the configs unchanged, 25 of
  57 golden outputs changed, 1,244 pixels in total, at near-ties (largest: `icon-calendar-out-3` 0.13 %,
  `05-mae-jemison-out-3` 0.08 %). The owner then chose to regenerate the configs (D7 b): only
  `07-alpha-hue` changed, because its farthest-point picks had a near-tie. At size 3 the third pick is now
  `[0,224,255,116]` / Pantone 3105 instead of `[255,0,2,116]` / Pantone 177, which changes 31.5 % of
  `07-alpha-hue-out-3`. Sizes 7 and 16 only reordered their picks: size 7's output is unchanged, and size
  16's 26 changed pixels come from the ΔE switch (counted in the 1,244), not from the reordering.

**Native vs WASM:** since R11, ΔE is bit-identical in native and WASM builds, so ties and near-ties
resolve the same way in the browser as in the CLI and goldens (see "Color math" above).

Each change updates the baseline snapshot (and, if pixels move, the goldens) in the same commit, so its
effect is a reviewed diff.
