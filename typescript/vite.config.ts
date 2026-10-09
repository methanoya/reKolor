// Vite settings: Vite is the development server (`npm run dev`, with instant reloads) and the
// production bundler (`npm run build`, output in `dist/`). The Vitest test runner reuses this
// file (`vitest.config.ts`).
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, searchForWorkspaceRoot } from 'vite';
import { wasmWatch } from './wasm-watch.ts';

// The base path is `/` by default. `vite build --mode pages` builds for GitHub Pages under
// `/reKolor/`; `REKOLOR_BASE` overrides both.
// `({ mode }) => ({...})` is an arrow function that returns the settings object; `mode` comes from
// `--mode` on the command line. `??` falls back to the right side when the left is `undefined`.
export default defineConfig(({ mode }) => ({
  base: process.env.REKOLOR_BASE ?? (mode === 'pages' ? '/reKolor/' : '/'),
  // `wasmWatch`: the dev server rebuilds the WASM package when its Rust sources change.
  // `svelte()` compiles `.svelte` files.
  plugins: [svelte(), wasmWatch()],
  // Bundle the Web Worker (`src/lib/worker.ts`) as an ES module, so it can use `import`.
  worker: { format: 'es' },
  // Pre-bundled up front: discovering them on first load makes Vite reload the page, which made
  // the first browser-test run time out.
  optimizeDeps: { include: ['comlink', 'vitest-browser-svelte'] },
  // Output modern JavaScript; every current browser supports ES2022.
  build: { target: 'es2022' },
  // The dev server may only serve files from these folders; everything else is refused.
  server: {
    fs: {
      // The palette (`../palettes/pantone.json`) and the WASM package (`../rust/crates/wasm/pkg`,
      // linked into node_modules) live outside `typescript/`; the browser tests also read the
      // decoder fixtures and sample images.
      allow: [
        searchForWorkspaceRoot(process.cwd()),
        '../palettes',
        '../rust/crates/wasm/pkg',
        '../rust/testdata/decoders',
        '../samples',
      ],
    },
  },
}));
