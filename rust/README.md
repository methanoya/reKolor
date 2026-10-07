# reKolor, Rust part

A self-contained Cargo workspace (R1, R2): the color engine, its browser interface, native file
I/O and a command-line tool. It builds and tests on its own; a web app consumes only the WASM
package it produces.

| Crate | What it is | Must not depend on |
|---|---|---|
| `crates/core` (`rekolor-core`) | Pure color logic on raw RGBA8 buffers: `recolor`, `analyze`, `Palette` (suggest, nearest), `pick` by coordinate, typed errors. No I/O, no printing. | wasm-bindgen, tsify, `image` |
| `crates/wasm` (`rekolor-wasm`) | Thin browser interface over `core` (wasm-bindgen + tsify). Classes `SourceImage` and `Palette`; results as `Outcome<T>` values. | `rekolor-io` (it would pull `image` into the WASM build) |
| `crates/io` (`rekolor-io`) | Native decoding of every readable `image` format to RGBA8 (EXIF orientation applied), PNG encoding, and the decoder-discrepancy warnings (M1) as data. | — |
| `crates/cli` (`rekolor-cli`, binary `rekolor`) | Command-line tool on top of `io` and `core`; also a small library (configs, palette loading, config generation, discovery) used by the golden test. | — |

Shared test data lives in `testdata/` (see [Baseline](#baseline-r9)); the golden set lives in
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
wasm-pack test --node crates/wasm                               # inside WASM: baseline, boundary, errors
cargo test --release -p rekolor-cli --test golden -- --ignored  # golden set (opt-in, ~15 s)
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
| core unit tests | `crates/core/src/*` | validation, errors, compositing, matching, ties, palette rule, pick |
| baseline | `crates/core/tests/baseline.rs` | `core` reproduces the recorded baseline pixel for pixel (R9) |
| suggestions | `crates/core/tests/suggestions.rs` | Pantone suggestions match the 2023 JavaScript (5,096 colors) |
| WASM | `crates/wasm/tests/node.rs` | the baseline inside WASM (native = WASM), malformed input → error values, lengths, factories, pick |
| contract | `crates/wasm/ts-test/` | the generated `.d.ts` (camelCase, `Outcome` narrowing, typed arrays) and runtime behavior from TypeScript |
| io | `crates/io/tests/io.rs` | lossless PNG round trip, 16-bit/grayscale, ICC and EXIF warnings, typed errors, all samples decode |
| CLI | `crates/cli/src/*`, `crates/cli/tests/cli.rs` | configs, generator rules, discovery; the real binary end to end |
| golden | `crates/cli/tests/golden.rs` | every sample matches its reviewed outputs (opt-in) |

## WASM package

```sh
wasm-pack build crates/wasm --release --target web   # → crates/wasm/pkg (ignores itself via pkg/.gitignore)
```

The package's `.d.ts` carries the whole contract. Usage from TypeScript (in the app this belongs in
a Web Worker, T7):

```ts
import init, { Palette, SourceImage } from 'rekolor-wasm';

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
```

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

## Baseline (R9)

The restructuring kept the existing output exactly. The reference was recorded from the existing
crate **after the dependency upgrade** (R12), not from the 2023 code itself.

| Path | What |
|---|---|
| `testdata/generator.rs` | Deterministic test buffers and mapping sets, plain Rust with no dependencies, included as a module wherever needed. Fixtures: `alpha_ramp` (every alpha 0–255 on 8 colors), `edges` (alpha- and color-anti-aliased edges on a transparent background), `picks` (exact pick colors and their ±1 neighbors), `gradient` (many unique colors), `noise` (random RGBA), `transparent` (fully transparent pixels with hidden RGB). Mapping sets: `none`, `one`, `calendar3`, `screenshot8`, `swapped4`. |
| `testdata/baseline/recolor/<fixture>__<mapping set>.png` | Output of `replace_rgb_colors` for every fixture × mapping set, stored exactly as returned (PNG). Compare **decoded pixels**, never bytes (X2). |
| `testdata/baseline/image-info.tsv` | Output of `image_info` for every fixture. |
| `testdata/baseline/pantone-suggestions.tsv` | The 2023 JavaScript Pantone suggestion (`typescript/src/palette.ts`, with `color-diff`) for 5,096 colors (a 16-level grid plus 1,000 pseudo-random colors), with its ΔE. Reference for the Rust suggestion (R10), compared with a near-tie tolerance. |

### How it was recorded

The baseline was recorded at commit `c0d094c` ("Record the baseline"), which contains the existing
crate's sources and the recorder (`rust/src/`, `rust/examples/record_baseline.rs`).
The recorder included the existing modules (`src/conv.rs`, `info.rs`, `utils.rs`, `console.rs`)
unchanged, the same way `src/main.rs` did. It encoded each fixture as a lossless PNG (checked by
decoding it again), then passed it to `image_info` and `replace_rgb_colors`.

The existing crate has since been removed; `crates/core` reproduces it (tests in
`crates/core/tests/baseline.rs` and `suggestions.rs`). The baseline is not re-recorded from the new
code: it is the fixed reference. If the inputs in `testdata/generator.rs` ever have to change,
re-record from that commit:

```sh
git worktree add ../rekolor-baseline c0d094c     # the existing crate + recorder
# copy the new testdata/generator.rs into it, then, inside ../rekolor-baseline/rust:
cargo run --release --example record_baseline     # recolor outputs + image-info.tsv
```

The JavaScript suggestions don't depend on the removed crate:

```sh
node testdata/record_pantone_suggestions.cjs      # pantone-suggestions.tsv (needs `npm ci` in typescript/)
```

### Verified when recorded (2026-10-07)

- Re-running both recorders produces byte-identical files.
- The existing crate's WASM build (`wasm-pack --target nodejs`), run in Node on the same fixture
  PNGs and mapping sets, returns byte-identical outputs for all 30 cases and identical
  `image_info` for all 6 fixtures.

## Behavior

The output still reproduces the existing behavior (R9), including its known issues (matching
against inks, alpha composited over white, the 1.5× white/black preference, …). They are recorded
as I1–I9 in the decision list and change one decision at a time; each change updates the baseline
expectations or the golden outputs in the same commit, so its effect is a reviewed diff.
