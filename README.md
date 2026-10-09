# reKolor

Preview how a picture prints with a limited ink palette, on a bag, a T-shirt, a wall or any other
material. Open an image and click the colors to keep: each one is replaced by its nearest
[Pantone](https://www.pantone.com/articles/color-palettes) ink, matched by
[CIEDE2000](https://en.wikipedia.org/wiki/Color_difference#CIEDE2000) color distance, and the
preview updates as you pick.

Try it at [methanoya.github.io/reKolor](https://methanoya.github.io/reKolor/).

![The web app: an illustration printed with eight inks on a sky-blue material](screenshot.png)

- **Picks:** every pixel takes the ink of the nearest picked color. Each pick suggests the nearest
  Pantone ink; the list offers the next closest ones. Shift-drag a pick's circle to move it. A
  pick's ΔE slider widens its reach: colors within that distance of it print with its ink.
- **Material:** the color of the garment or surface. Colors are matched as they look on it, and the
  preview shows it behind the print.
- **Not printed:** colors the material already has need no ink. Pixels within a chosen distance of
  them stay transparent in the downloaded PNG, so the material shows through.
- **Ink palette:** the built-in Pantone inks, or your own: upload a palette file (JSON, described by
  `palettes/palette.schema.json`) and every pick takes its nearest ink from it. Download the current
  palette, or go back to Pantone, from the same line.
- **Reloads keep your work:** the image, picks, material and palette stay in your browser for the
  tab, so reloading the page brings them back.
- **Palette configs:** export the picks, the material and the unprinted colors as a
  `*.palettes.toml` file and import them again, in the app or with the `rekolor` command-line tool.

The color engine is written in Rust and runs in the browser as WebAssembly, in a Web Worker; the
web app is Svelte and TypeScript.

## Project layout

| Folder | What |
|---|---|
| `rust/` | The color engine as a Cargo workspace: pure core, WASM interface, file I/O, palette configs and the `rekolor` CLI. See [rust/README.md](rust/README.md). |
| `typescript/` | The web app (Svelte, TypeScript, Vite), running the WASM engine in a Web Worker. See [typescript/README.md](typescript/README.md). |
| `palettes/` | The Pantone palette (`pantone.json`). |
| `samples/` | The golden set: sample images, their palette configs and the reviewed outputs. |
