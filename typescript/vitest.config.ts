import { playwright } from '@vitest/browser-playwright';
import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config.ts';
import { mouseCommands } from './tests/browser/mouse.commands.ts';

export default defineConfig((env) =>
  mergeConfig(viteConfig(env), {
    test: {
      projects: [
        {
          extends: true,
          test: { name: 'node', include: ['src/**/*.test.ts'], environment: 'node' },
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
