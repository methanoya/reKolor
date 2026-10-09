// ESLint settings (`npm run lint`): ESLint reports likely bugs and bad patterns. Formatting is
// left to prettier, which `npm run lint` also checks.
import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import { defineConfig } from 'eslint/config';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import ts from 'typescript-eslint';
import svelteConfig from './svelte.config.js';

// A list of config objects; later entries add to or override earlier ones.
export default defineConfig(
  { ignores: ['dist/', 'node_modules/', 'test-results/', '.vitest/'] },
  // The recommended rules for JavaScript, TypeScript and Svelte; `prettier` then turns off every
  // rule that would conflict with prettier's formatting.
  js.configs.recommended,
  ts.configs.recommended,
  svelte.configs.recommended,
  prettier,
  svelte.configs.prettier,
  // Global names that exist without an import (`window`, `process`, …): code here runs in the
  // browser and in Node.js.
  { languageOptions: { globals: { ...globals.browser, ...globals.node } } },
  {
    // Svelte files: parse the `<script lang="ts">` blocks with the TypeScript parser, with full
    // type information (`projectService`).
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        parser: ts.parser,
        svelteConfig,
      },
    },
  },
);
