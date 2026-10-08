import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, searchForWorkspaceRoot } from 'vite';

// The base path is `/` by default. `vite build --mode pages` builds for GitHub Pages under `/reKolor/`
// (W14 c: deployment itself is decided later); `REKOLOR_BASE` overrides both.
export default defineConfig(({ mode }) => ({
  base: process.env.REKOLOR_BASE ?? (mode === 'pages' ? '/reKolor/' : '/'),
  plugins: [svelte()],
  worker: { format: 'es' },
  // Pre-bundled up front: discovering them on first load makes Vite reload the page, which made
  // the first browser-test run time out.
  optimizeDeps: { include: ['comlink', 'vitest-browser-svelte'] },
  build: { target: 'es2022' },
  server: {
    fs: {
      // The palette (`../palettes/pantone.json`) and the WASM package (`../rust/crates/wasm/pkg`,
      // linked into node_modules) live outside `typescript/`; the browser tests also read the decoder
      // fixtures and sample images.
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
