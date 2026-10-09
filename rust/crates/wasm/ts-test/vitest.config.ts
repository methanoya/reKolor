// Vitest settings: run the `*.test.ts` files in Node.js. (`types.check.ts` is only type-checked by
// `tsc`, never run.) `defineConfig` adds type checking and editor completion to the settings
// object.
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['test/**/*.test.ts'],
    environment: 'node',
  },
});
