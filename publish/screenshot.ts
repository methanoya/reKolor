// Recreates the README screenshot (`screenshot.png` at the project root): the web app with an image
// from this folder open and its palette config imported. How and when to run it: `screenshot.md`.
//
//   node publish/screenshot.ts
//
// Builds the web app, serves the build locally, opens it in Chromium, and writes the whole page to
// `screenshot.png`. Exits non-zero if a step fails.
// Node 24 runs this `.ts` file directly, stripping its erasable types before execution.

import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Browser, Locator } from '../typescript/node_modules/playwright/index.js';

// Paths, relative to this file's folder.
const here = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));
const web = here('../typescript/');
const out = here('../screenshot.png');
const port = 4180;
const url = `http://127.0.0.1:${port}/`;

// The first image in this folder that has a palette config of the same name: `<name>.jpg` or
// `<name>.png`, with `<name>.palettes.toml`. "First" is by file name, in character-code order
// (`sort()` without a comparer), so the choice doesn't depend on the system's language.
const pair = fs
  .readdirSync(here('.'))
  .sort()
  .map((file) => /^(.+)\.(jpg|png)$/.exec(file))
  .find((match) => match && fs.existsSync(here(`${match[1]}.palettes.toml`)));
if (!pair) {
  // The folder's name as it is, wherever the script lives.
  const folder = path.basename(here('.'));
  console.error(`${folder}/ has no image (<name>.jpg or <name>.png) with a <name>.palettes.toml`);
  process.exit(1);
}
const image = here(pair[0]);
const config = here(`${pair[1]}.palettes.toml`);
console.log(`image: ${pair[0]}, config: ${pair[1]}.palettes.toml`);

// Playwright is a dependency of the web app, not of this folder: load it from there.
const { chromium } = createRequire(`${web}package.json`)(
  'playwright',
) as typeof import('../typescript/node_modules/playwright/index.js');

// The production build, served at the root (`/`). Vite is run directly, not through `npm` or `npx`,
// so stopping it at the end stops the server itself.
const built = spawnSync('npm', ['run', 'build'], {
  cwd: web,
  stdio: 'inherit',
});
if (built.status !== 0) process.exit(built.status ?? 1);
const server = spawn(
  process.execPath,
  [
    `${web}node_modules/vite/bin/vite.js`,
    'preview',
    ...['--host', '127.0.0.1', '--port', String(port), '--strictPort'],
  ],
  { cwd: web, stdio: 'inherit' },
);

// `finally` stops the browser and the server whether the capture worked or not.
try {
  await waitForServer();
  const browser = await chromium.launch();
  try {
    await capture(browser);
  } finally {
    await browser.close();
  }
} finally {
  server.kill();
}

/** Opens the app, opens the image, imports the config, and writes the screenshot. */
async function capture(browser: Browser) {
  // The size the screenshot is taken at: 1280 CSS pixels wide, one device pixel each, light theme.
  // The capture covers the whole page, so its height is whatever the page needs.
  const page = await browser.newPage({
    viewport: { width: 1280, height: 900 },
    deviceScaleFactor: 1,
    colorScheme: 'light',
  });
  page.on('console', (m) => m.type() === 'error' && console.error(`console: ${m.text()}`));
  page.on('pageerror', (e) => console.error(`page error: ${e.message}`));

  // Waits for `target`; if the app shows an error first (a file it can't read, say), stops with the
  // app's message instead of a timeout.
  const alert = page.getByRole('alert');
  const shown = async (target: Locator, timeout = 20_000) => {
    await target.or(alert).first().waitFor({ timeout });
    if (await alert.isVisible()) throw new Error(`the app says: ${await alert.textContent()}`);
  };

  // The user's path: wait for the engine, open the image ("Open image…"), import the config
  // ("Import picks…"), and wait until the preview is drawn. Both file inputs are hidden behind
  // their buttons; setting their files is what choosing a file does.
  await page.goto(url);
  await shown(page.getByText(/Engine ready/));
  await page.locator('#file').setInputFiles(image);
  await shown(page.getByText('No picks yet: nothing is printed'));
  await page.locator('input[accept^=".toml"]').setInputFiles(config);
  // A config with several palettes asks which one to import: take the first one listed.
  const status = page.getByTestId('status').filter({ hasText: /^Imported / });
  const dialog = page.getByRole('dialog', { name: 'Which palette?' });
  await shown(status.or(dialog));
  if (await dialog.isVisible()) {
    await dialog.getByRole('button').first().click();
    await shown(status);
  }
  // The preview's heading once no recolor is running (while one is, it ends in "· updating…").
  // A recolor starts in the same update as the status line above, so it can't be missed.
  await shown(
    page.getByText(
      /^(Printed with \d+ inks?( · \d+ colors? left to the material)?|No picks yet: nothing is printed)$/,
    ),
    30_000,
  );

  console.log(`status: ${(await status.textContent())?.trim()}`);
  console.log(`file: ${(await page.getByTestId('file-info').textContent())?.trim()}`);
  await page.screenshot({ path: out, fullPage: true });
  console.log(`wrote ${out}`);
}

/** Waits until the preview server answers (it takes a moment to start). */
async function waitForServer() {
  for (let tries = 0; tries < 100; tries++) {
    try {
      if ((await fetch(url)).ok) return;
    } catch {
      // Not listening yet.
    }
    await new Promise<void>((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`the preview server didn't start on port ${port}`);
}
