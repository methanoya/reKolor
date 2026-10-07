# reKolor, Rust part

## Baseline (R9)

The restructuring must keep the existing output exactly. The reference is recorded from the
existing crate **after the dependency upgrade** (R12), not from the 2023 code itself.

| Path | What |
|---|---|
| `testdata/generator.rs` | Deterministic test buffers and mapping sets, plain Rust with no dependencies, included as a module wherever needed. Fixtures: `alpha_ramp` (every alpha 0–255 on 8 colors), `edges` (alpha- and color-anti-aliased edges on a transparent background), `picks` (exact pick colors and their ±1 neighbors), `gradient` (many unique colors), `noise` (random RGBA), `transparent` (fully transparent pixels with hidden RGB). Mapping sets: `none`, `one`, `calendar3`, `screenshot8`, `swapped4`. |
| `testdata/baseline/recolor/<fixture>__<mapping set>.png` | Output of `replace_rgb_colors` for every fixture × mapping set, stored exactly as returned (PNG). Compare **decoded pixels**, never bytes (X2). |
| `testdata/baseline/image-info.tsv` | Output of `image_info` for every fixture. |
| `testdata/baseline/pantone-suggestions.tsv` | The 2023 JavaScript Pantone suggestion (`typescript/src/palette.ts`, with `color-diff`) for 5,096 colors (a 16-level grid plus 1,000 pseudo-random colors), with its ΔE. Reference for the Rust suggestion (R10), compared with a near-tie tolerance. |

### Re-recording

Only when the inputs change on purpose (then in the same change as `testdata/generator.rs`):

```sh
cargo run --release --example record_baseline     # recolor outputs + image-info.tsv
node testdata/record_pantone_suggestions.cjs      # pantone-suggestions.tsv (needs `npm ci` in typescript/)
```

`examples/record_baseline.rs` includes the existing modules (`src/conv.rs`, `info.rs`, `utils.rs`,
`console.rs`) unchanged, the same way `src/main.rs` does. Each fixture is encoded as a lossless PNG
(checked by decoding it again), then passed to `image_info` and `replace_rgb_colors`.
`--inputs <dir>` also writes the fixture PNGs, for feeding the WASM build.

### Verified when recorded (2026-10-07)

- Re-running both recorders produces byte-identical files.
- The existing crate's WASM build (`wasm-pack --target nodejs`), run in Node on the same fixture
  PNGs and mapping sets, returns byte-identical outputs for all 30 cases and identical
  `image_info` for all 6 fixtures.
