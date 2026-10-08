// End-to-end smoke test (plan step 5): the real app in each engine — open → pick → live recolor →
// download — plus a config import.

import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';
import App from '../../src/App.svelte';
import '../../src/app.css';
import { fixtureFile } from './fixtures';

afterEach(() => vi.restoreAllMocks());

/** Captures the next download the app starts (it clicks a temporary <a download>). */
function captureDownload(): Promise<{ name: string; blob: Blob }> {
  return new Promise((resolve) => {
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      const name = this.download;
      void fetch(this.href)
        .then((r) => r.blob())
        .then((blob) => resolve({ name, blob }));
    });
  });
}

describe('reKolor app', () => {
  test('open → pick → recolor → download', async () => {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();

    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await fixtureFile('opaque'));
    await expect.element(screen.getByTestId('file-info')).toMatchTextContent('48 × 32');
    await expect
      .element(screen.getByText('No picks yet: the image as printed on white'))
      .toBeVisible();

    // Zoom to 1:1 so the click maps to an exact pixel, then click the canvas center.
    await screen.getByRole('button', { name: '1:1' }).click();
    const canvas = screen.container.querySelector('canvas')!;
    const box = canvas.getBoundingClientRect();
    await userEvent.click(page.elementLocator(canvas), {
      position: { x: box.width / 2, y: box.height / 2 },
    });
    await expect.element(screen.getByText('Printed with 1 ink')).toBeVisible();
    await expect
      .element(screen.getByRole('list', { name: 'Picked colors' }).getByRole('listitem'))
      .toHaveLength(1);
    // Picked at 100 %: the color on screen matched the stored pixel, so no warning.
    expect(screen.container.querySelectorAll('[role=note]')).toHaveLength(0);

    const download = captureDownload();
    await screen.getByTestId('download').click();
    const { name, blob } = await download;
    expect(name).toBe('opaque-rekolor.png');
    const head = new Uint8Array(await blob.slice(0, 24).arrayBuffer());
    expect(String.fromCharCode(...head.subarray(1, 4))).toBe('PNG');
    const view = new DataView(head.buffer);
    expect([view.getUint32(16), view.getUint32(20)]).toEqual([48, 32]);
  });

  test('a config import replaces the picks and recolors', async () => {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await fixtureFile('opaque'));
    await expect
      .element(screen.getByText('No picks yet: the image as printed on white'))
      .toBeVisible();

    const config = new File(
      [
        '[[palette]]\nsize = 2\npicks = [\n' +
          '  { rgba = [0, 0, 0, 255], ink = "Pantone 285" },\n' +
          '  { rgba = [255, 255, 255, 255], ink = "Pure White (non-palette)" },\n]\n',
      ],
      'two.palettes.toml',
    );
    const configInput = screen.container.querySelector<HTMLInputElement>('input[accept^=".toml"]')!;
    await userEvent.upload(page.elementLocator(configInput), config);
    await expect.element(screen.getByText('Printed with 2 inks')).toBeVisible();
    await expect
      .element(screen.getByTestId('status'))
      .toMatchTextContent('Imported 2 picks (size 2)');
  });

  test('a failed replacement keeps the previous image usable (review fix)', async () => {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await fixtureFile('opaque'));
    await expect
      .element(screen.getByText('No picks yet: the image as printed on white'))
      .toBeVisible();

    const junk = new File([new Uint8Array([1, 2, 3, 4])], 'broken.png', { type: 'image/png' });
    await userEvent.upload(page.elementLocator(input), junk);
    await expect.element(screen.getByRole('alert')).toMatchTextContent('broken.png');
    await expect
      .element(screen.getByTestId('status'))
      .toHaveTextContent('Kept the previous image.');

    // The kept image still picks, recolors and downloads.
    await screen.getByRole('button', { name: '1:1' }).click();
    const canvas = screen.container.querySelector('canvas')!;
    const box = canvas.getBoundingClientRect();
    await userEvent.click(page.elementLocator(canvas), {
      position: { x: box.width / 2, y: box.height / 2 },
    });
    await expect.element(screen.getByText('Printed with 1 ink')).toBeVisible();
    const download = captureDownload();
    await screen.getByTestId('download').click();
    expect((await download).name).toBe('opaque-rekolor.png');
  });

  test('imports apply in the order they were made (review fix)', async () => {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await fixtureFile('opaque'));
    await expect
      .element(screen.getByText('No picks yet: the image as printed on white'))
      .toBeVisible();

    const config = (inks: string[]) =>
      new File(
        [
          `[[palette]]\nsize = ${inks.length}\npicks = [\n` +
            inks.map((ink, i) => `  { rgba = [${i}, 0, 0, 255], ink = "${ink}" },\n`).join('') +
            ']\n',
        ],
        `${inks.length}.palettes.toml`,
      );
    const configInput = screen.container.querySelector<HTMLInputElement>('input[accept^=".toml"]')!;
    // Two imports back to back: the second one made must be the one that stays.
    await Promise.all([
      userEvent.upload(
        page.elementLocator(configInput),
        config(['Pantone 185', 'Pantone 285', 'Pantone 300']),
      ),
      userEvent.upload(page.elementLocator(configInput), config(['Pantone 285'])),
    ]);
    await expect.element(screen.getByText('Printed with 1 ink')).toBeVisible();
    await expect
      .element(screen.getByTestId('status'))
      .toHaveTextContent('Imported 1 pick (size 1) from 1.palettes.toml.');
  });
});
