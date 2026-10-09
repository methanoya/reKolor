// Svelte compiler settings, shared by Vite, `svelte-check` and ESLint. `vitePreprocess` lets
// `.svelte` files use TypeScript (`<script lang="ts">`): Vite strips the types before the Svelte
// compiler sees the code.
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
};
