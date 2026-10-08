# reKolor

This made as a printing preview tool for limited palette pictures on bags, t-shirts, walls, etc. 
For now, it works with [Pantone](https://www.pantone.com/articles/color-palettes) palette. The good color match is based on [CIEDE2000](https://en.wikipedia.org/wiki/Color_difference#CIEDE2000) color distance.

See the tool on [https://methanoya.github.io/reKolor/](https://methanoya.github.io/reKolor/)

It uses Typescript, Rust, WebAssembly, Svelte


![screenshot.png](screenshot.png)

## Project layout

| Folder | What |
|---|---|
| `rust/` | The color engine as a Cargo workspace: pure core, WASM interface, file I/O, palette configs and the `rekolor` CLI. See [rust/README.md](rust/README.md). |
| `typescript/` | The web app (Svelte 5, TypeScript, Vite), running the WASM engine in a Web Worker. See [typescript/README.md](typescript/README.md). |
| `palettes/` | The Pantone palette (`pantone.json`). |
| `samples/` | The golden set: sample images, their palette configs and the reviewed outputs. |

