// Production smoke test under the GitHub Pages base path: builds with `--mode pages`, serves
// `dist/` at `/reKolor/`, and drives the app in Chromium. Exits non-zero on any failure, console
// error or failed request.
//
//   npm run smoke

// A plain Node.js script (`.mjs`: a JavaScript module), so it can use `await` at the top level.
// The `/** @type {...} */` comments give TypeScript the variables' types, since this file is
// JavaScript (`tsconfig.json` type-checks it).
import { build, preview } from 'vite';
import { chromium } from 'playwright';

const sample = new URL('../../samples/good-looking/04-tiger.png', import.meta.url).pathname;

// Build exactly as for GitHub Pages, then serve the result locally with `vite preview`.
await build({ mode: 'pages', logLevel: 'warn' });
const server = await preview({ mode: 'pages', preview: { port: 4174, strictPort: true } });
const url = 'http://localhost:4174/reKolor/';
/** @type {string[]} */
const problems = [];
const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  // Record every problem the page reports instead of stopping at the first one.
  page.on('console', (m) => m.type() === 'error' && problems.push(`console: ${m.text()}`));
  page.on('pageerror', (e) => problems.push(`page error: ${e.message}`));
  page.on('requestfailed', (r) => problems.push(`request failed: ${r.url()}`));
  page.on('response', (r) => r.status() >= 400 && problems.push(`HTTP ${r.status()}: ${r.url()}`));
  // Network requests include the worker's (the WASM is fetched inside the worker).
  /** @type {string[]} */
  const requested = [];
  page.on('request', (r) => requested.push(new URL(r.url()).pathname));

  // The user's path: wait for the engine, open a sample image, click it once (a pick), and wait for
  // the status texts the app shows after each step.
  await page.goto(url);
  await page.getByText(/Engine ready/).waitFor({ timeout: 15_000 });
  await page.locator('#file').setInputFiles(sample);
  await page.getByText('No picks yet: nothing is printed').waitFor({ timeout: 15_000 });
  const canvas = page.locator('canvas').first();
  const box = await canvas.boundingBox();
  if (!box) throw new Error('the original canvas has no size');
  await page.mouse.click(box.x + box.width * 0.3, box.y + box.height * 0.55);
  await page.getByText('Printed with 1 ink').waitFor({ timeout: 15_000 });

  // Every file must come from under the base path; one that doesn't would fail on GitHub Pages.
  const assets = requested.filter((p) => p !== '/reKolor/');
  const outside = assets.filter((p) => !p.startsWith('/reKolor/'));
  if (outside.length) problems.push(`assets outside /reKolor/: ${outside.join(', ')}`);
  for (const kind of ['.wasm', 'worker']) {
    if (!assets.some((p) => p.includes(kind))) problems.push(`no ${kind} asset was loaded`);
  }
  console.log(`loaded ${assets.length} assets under /reKolor/ (worker, WASM, palette included)`);
} catch (e) {
  problems.push(String(e));
} finally {
  await browser.close();
  await new Promise((resolve) => server.httpServer.close(resolve));
}

if (problems.length) {
  console.error(`smoke test FAILED:\n  ${problems.join('\n  ')}`);
  process.exit(1);
}
console.log(`smoke test passed: ${url} opens an image, picks and recolors`);
