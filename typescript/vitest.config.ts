// Vitest (the test runner) settings, built on top of the Vite settings. Two test projects:
// - `node`: unit tests next to the code (`src/**/*.test.ts`, and `wasm-watch.test.ts` for the dev
//   server plugin), run in Node.js (`npm test`);
// - `browser`: tests in real browsers driven by Playwright (`npm run test:browser`), which mount
//   the actual app and use the real WASM package in a real Web Worker.
import { playwright } from '@vitest/browser-playwright';
import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config.ts';
import { mouseCommands } from './tests/browser/mouse.commands.ts';

export default defineConfig((env) =>
  mergeConfig(viteConfig(env), {
    test: {
      projects: [
        {
          // `extends: true`: inherit the shared settings above (the Vite config).
          extends: true,
          test: { name: 'node', include: ['src/**/*.test.ts', '*.test.ts'], environment: 'node' },
        },
        {
          extends: true,
          test: {
            name: 'browser',
            include: ['tests/browser/**/*.test.ts'],
            testTimeout: 30_000,
            browser: {
              enabled: true,
              headless: true,
              provider: playwright(),
              // Extra commands the tests can call to drive the real mouse
              // (`tests/browser/mouse.commands.ts`).
              commands: mouseCommands,
              // All three Playwright engines (WebKit is not branded Safari).
              instances: [{ browser: 'chromium' }, { browser: 'firefox' }, { browser: 'webkit' }],
            },
          },
        },
      ],
    },
  }),
);
