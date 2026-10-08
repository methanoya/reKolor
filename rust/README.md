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
| baseline | `crates/core/tests/baseline.rs` | `core` matches the baseline snapshot pixel for pixel |
| suggestions | `crates/core/tests/suggestions.rs` | Pantone suggestions match the snapshot exactly (5,096 distinct colors) |
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

## Baseline snapshot (R9, P1)

`testdata/baseline/` holds the **current approved behavior** of `core` on generated inputs. Tests only
read it; after an intentional change, regenerate it and review the diff:

```sh
cargo run -p rekolor-core --example update_baseline                          # rewrite what changed
cargo run -p rekolor-core --example update_baseline -- --colors-from-current # same, keeping the suggestion colors
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

**Native vs WASM:** CIEDE2000 values differ slightly between native and WASM builds (about 1e-4 ΔE,
measured), so a pixel whose two nearest inks are within about 1e-3 ΔE can get a different ink in the
browser than in the CLI and goldens. The baseline and the WASM suite contain no such near-ties; the
exact-tie test is native-only.

Each change updates the baseline snapshot (and, if pixels move, the goldens) in the same commit, so its
effect is a reviewed diff.
