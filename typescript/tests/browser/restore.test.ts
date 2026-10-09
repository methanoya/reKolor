// Restoring a tab's saved work at startup (`restore` in `src/App.svelte`, storage in
// `src/lib/persist.ts`), where it can go wrong: the saved image is gone, or the user opens an image
// before the saved one has loaded. `vi.mock` with `spy: true` keeps the storage functions real and
// lets a test hold back `loadImage`.
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';
import App from '../../src/App.svelte';
import '../../src/app.css';
import { loadImage, saveImage } from '../../src/lib/persist';
import { fixtureFile } from './fixtures';

vi.mock('../../src/lib/persist', { spy: true });

afterEach(() => vi.mocked(loadImage).mockReset());

const STATE_KEY = 'rekolor.session';

/** A saved session on a black material, for the image named `image` (no picks). */
const savedSession = (image: { name: string; size: number }) =>
  JSON.stringify({
    version: 1,
    image,
    material: { r: 0, g: 0, b: 0 },
    materialChosen: true,
    picks: [],
    ranges: [],
  });

/** A small PNG of one color. */
async function pngFile(name: string): Promise<File> {
  const canvas = new OffscreenCanvas(8, 8);
  const context = canvas.getContext('2d')!;
  context.fillStyle = 'rgb(20 160 90)';
  context.fillRect(0, 0, 8, 8);
  return new File([await canvas.convertToBlob({ type: 'image/png' })], name, { type: 'image/png' });
}

describe('restoring the saved work', () => {
  test("a session whose image can't come back is kept until the next change", async () => {
    // Saved with an image that isn't stored (IndexedDB is empty).
    const saved = savedSession({ name: 'gone.png', size: 10 });
    sessionStorage.setItem(STATE_KEY, saved);
    const screen = await render(App);
    // The material comes back; the image can't.
    await expect.element(screen.getByTestId('material')).toHaveValue('#000000');
    await expect.poll(() => vi.mocked(loadImage).mock.settledResults.length).toBe(1);
    await new Promise((resolve) => setTimeout(resolve, 300));
    expect(sessionStorage.getItem(STATE_KEY)).toBe(saved);

    // The next change saves. First the change itself (so a click that didn't land shows as that),
    // then the save.
    await screen.getByRole('button', { name: 'White' }).click();
    await expect.element(screen.getByTestId('status')).toHaveTextContent('Material #ffffff.');
    await expect
      .poll(
        () => (JSON.parse(sessionStorage.getItem(STATE_KEY)!) as { material: unknown }).material,
      )
      .toEqual({ r: 255, g: 255, b: 255 });
  });

  test('an image opened while the saved one loads wins over it', async () => {
    const savedFile = await pngFile('saved.png');
    expect(await saveImage(savedFile)).toBe(true);
    sessionStorage.setItem(STATE_KEY, savedSession({ name: 'saved.png', size: savedFile.size }));
    // Loading the saved image is held back until the test releases it.
    let release!: () => void;
    const held = new Promise<void>((resolve) => (release = resolve));
    vi.mocked(loadImage).mockImplementationOnce(async () => {
      await held;
      return savedFile;
    });

    const screen = await render(App);
    await expect.poll(() => vi.mocked(loadImage).mock.calls.length).toBe(1);
    // Meanwhile the user opens another image.
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    const opaque = await fixtureFile('opaque');
    await userEvent.upload(page.elementLocator(input), opaque);
    await expect.element(screen.getByTestId('file-info')).toMatchTextContent('48 × 32');

    // The saved image arrives too late: the user's stays.
    release();
    await expect.poll(() => vi.mocked(loadImage).mock.settledResults.length).toBe(1);
    await new Promise((resolve) => setTimeout(resolve, 500));
    await expect.element(screen.getByTestId('file-info')).toMatchTextContent(opaque.name);
    await expect.element(screen.getByTestId('status')).not.toMatchTextContent(/Restored/);
  });
});
