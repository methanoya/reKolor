// End-to-end tests: the real app in each engine — open → pick → live recolor →
// download — plus a config import.

import { commands, page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { afterEach, describe, expect, test, vi } from 'vitest';
import App from '../../src/App.svelte';
import '../../src/app.css';
import { fixtureFile } from './fixtures';
import type {} from './mouse.commands';

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
    await expect.element(screen.getByText('No picks yet: nothing is printed')).toBeVisible();

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
    await expect.element(screen.getByText('No picks yet: nothing is printed')).toBeVisible();

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

  test('a failed replacement keeps the previous image usable', async () => {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await fixtureFile('opaque'));
    await expect.element(screen.getByText('No picks yet: nothing is printed')).toBeVisible();

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

  test('imports apply in the order they were made', async () => {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await fixtureFile('opaque'));
    await expect.element(screen.getByText('No picks yet: nothing is printed')).toBeVisible();

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

  // ── Moving picks ───────────────────────────────────────────────────────────────────────────────
  // At 1:1 the canvas center is column 24 of the 48 × 32 test images, and each CSS pixel to the
  // right is one column more. The row is read from the first pick (it differs by engine).

  const ORIGINAL = 'canvas[aria-label^="Original"]';
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

  /** 48 × 32, columns 0–23 one flat color and 24–47 another (for same-color moves). */
  async function halvesFile(): Promise<File> {
    const canvas = new OffscreenCanvas(48, 32);
    const context = canvas.getContext('2d')!;
    context.fillStyle = 'rgb(200 40 40)';
    context.fillRect(0, 0, 24, 32);
    context.fillStyle = 'rgb(40 40 200)';
    context.fillRect(24, 0, 24, 32);
    return new File([await canvas.convertToBlob({ type: 'image/png' })], 'halves.png');
  }

  /** Opens an image at 1:1 and picks at each offset (CSS px right of the canvas center). */
  async function opened(file: File, offsets: number[]) {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), file);
    await expect.element(screen.getByText('No picks yet: nothing is printed')).toBeVisible();
    await screen.getByRole('button', { name: '1:1' }).click();
    const canvas = screen.container.querySelector<HTMLCanvasElement>(ORIGINAL)!;
    const box = canvas.getBoundingClientRect();
    const at = (dx: number) => ({ x: box.width / 2 + dx, y: box.height / 2 });
    const items = screen.getByRole('list', { name: 'Picked colors' }).getByRole('listitem');
    const click = async (dx: number) => {
      const count = screen.container.querySelectorAll('.pick').length;
      await userEvent.click(page.elementLocator(canvas), { position: at(dx) });
      await expect.element(items).toHaveLength(count + 1);
    };
    for (const dx of offsets) await click(dx);

    /** The picks' colors as shown in the list, in order. */
    const colors = () =>
      [...screen.container.querySelectorAll('.pick .color')].map((e) => e.textContent!.trim());
    const selects = () => [...screen.container.querySelectorAll<HTMLSelectElement>('.pick select')];
    /** A held mouse: press at, move to, release (Shift is held separately). */
    const mouse = {
      down: async (dx: number) => {
        await commands.mouseTo(ORIGINAL, at(dx).x, at(dx).y);
        await commands.mouseDown();
      },
      to: (dx: number) => commands.mouseTo(ORIGINAL, at(dx).x, at(dx).y),
      up: () => commands.mouseUp(),
    };
    const shift = (held: boolean) => userEvent.keyboard(held ? '{Shift>}' : '{/Shift}');
    /** The pointer id of the next press on the canvas. */
    const nextPointerId = () =>
      new Promise<number>((resolve) =>
        canvas.addEventListener('pointerdown', (e) => resolve(e.pointerId), { once: true }),
      );
    /** A pointer event from the page's script, at an offset from the canvas center. */
    const synthetic = (type: string, pointerId: number, dx: number) =>
      canvas.dispatchEvent(
        new PointerEvent(type, {
          pointerId,
          bubbles: true,
          shiftKey: true,
          clientX: box.left + at(dx).x,
          clientY: box.top + at(dx).y,
        }),
      );
    /**
     * Lets the canvas capture pointers made by `synthetic`, as the browser does for a real second
     * pointer (pen + touch); otherwise `setPointerCapture` throws for them before the app reacts.
     */
    const acceptSyntheticCapture = () => {
      const real = canvas.setPointerCapture.bind(canvas);
      vi.spyOn(canvas, 'setPointerCapture').mockImplementation((id) => {
        try {
          real(id);
        } catch {
          // a synthetic pointer: treated as captured
        }
      });
    };
    const status = screen.getByTestId('status');
    return {
      acceptSyntheticCapture,
      screen,
      canvas,
      at,
      click,
      colors,
      selects,
      mouse,
      shift,
      nextPointerId,
      synthetic,
      status,
    };
  }

  /** The row's green value from a list label "rgb r, g, b". */
  const green = (label: string | undefined) => /^rgb \d+, (\d+),/.exec(label ?? '')?.[1];

  test('a held Shift-drag moves the pick live, before the release', async () => {
    const { colors, mouse, shift, status } = await opened(await fixtureFile('opaque'), [0]);
    const g = green(colors()[0]);
    expect(colors()[0]).toMatch(/^rgb 130, /);
    await shift(true);
    await mouse.down(0);
    await mouse.to(10);
    await expect.poll(() => colors()[0]).toMatch(new RegExp(`^rgb 184, ${g}, `));
    expect(status.element().textContent).not.toContain('Moved to'); // still held
    await mouse.up();
    await shift(false);
    await expect.element(status).toMatchTextContent(`Moved to (184, ${g},`);
  });

  test('a hand-chosen ink stays within its color and is replaced by a new one', async () => {
    const { screen, colors, selects, mouse, shift, status } = await opened(
      await halvesFile(),
      [-6],
    );
    expect(colors()).toEqual(['rgb 200, 40, 40']);
    await userEvent.selectOptions(selects()[0]!, '3');
    await expect.poll(() => selects()[0]!.selectedIndex).toBe(3);
    const hand = selects()[0]!.selectedOptions[0]!.text;

    await shift(true);
    await mouse.down(-6);
    await mouse.to(-3); // column 21: the same color
    await mouse.up();
    await expect.element(status).toMatchTextContent('Moved to (200, 40, 40)');
    expect(selects()[0]!.selectedIndex).toBe(3); // the hand choice is kept

    await mouse.down(-3);
    await mouse.to(6); // column 30: the other color
    await mouse.up();
    await shift(false);
    await expect.element(status).toMatchTextContent('Moved to (40, 40, 200)');
    expect(colors()).toEqual(['rgb 40, 40, 200']);
    expect(selects()[0]!.selectedIndex).toBe(0); // the nearest ink for the new color
    expect(selects()[0]!.selectedOptions[0]!.text).not.toBe(hand);
    await expect.element(screen.getByText('Printed with 1 ink')).toBeVisible();
  });

  test('a drop on a picked color leaves the pick on the last free pixel', async () => {
    const { colors, mouse, shift, status } = await opened(await fixtureFile('opaque'), [0, 10]);
    const [a, b] = colors();
    const g = green(a);
    expect(b).toMatch(/^rgb 184, /);
    await shift(true);
    await mouse.down(10);
    await mouse.to(5); // column 29: free
    await expect.poll(() => colors()[1]).toMatch(new RegExp(`^rgb 157, ${g}, `));
    await mouse.to(0); // column 24: pick A's color
    await mouse.up();
    await shift(false);
    await expect.element(status).toMatchTextContent('already picked');
    expect(colors()[0]).toBe(a);
    expect(colors()[1]).toMatch(new RegExp(`^rgb 157, ${g}, `));
  });

  test('away from the circles, a Shift-drag pans and a Shift-click picks', async () => {
    const { colors, mouse, shift } = await opened(await fixtureFile('opaque'), [0]);
    const before = colors();
    await shift(true);
    await mouse.down(20); // 20 CSS px from the circle: out of reach
    await mouse.to(30); // pans the image 10 px right
    await mouse.up();
    expect(colors()).toEqual(before);
    // A Shift-click 20 px from the moved circle: column 24 − 10 − 10 = 4 after the pan.
    await mouse.down(-10);
    await mouse.up();
    await shift(false);
    await expect.poll(() => colors()[1]).toMatch(/^rgb 21, /);
  });

  test('a move answers only to the pointer that started it', async () => {
    const {
      colors,
      mouse,
      shift,
      status,
      nextPointerId,
      synthetic,
      click,
      acceptSyntheticCapture,
    } = await opened(await fixtureFile('opaque'), [0]);
    acceptSyntheticCapture();
    const g = green(colors()[0]);
    await shift(true);
    const id = nextPointerId();
    await mouse.down(0);
    await mouse.to(5);
    await expect.poll(() => colors()[0]).toMatch(/^rgb 157, /);
    const other = (await id) + 100;
    // A second pointer pressing (with Shift) on the moved circle and away from it, moving,
    // releasing, being cancelled and losing capture: none of it replaces the move or pans.
    synthetic('pointerdown', other, 5);
    synthetic('pointerdown', other + 1, -15);
    synthetic('pointermove', other, 15);
    synthetic('pointermove', other + 1, -5);
    for (const type of ['pointerup', 'pointercancel', 'lostpointercapture']) {
      synthetic(type, other, 15);
      synthetic(type, other + 1, -5);
    }
    await wait(300);
    expect(colors()[0]).toMatch(/^rgb 157, /);
    expect(status.element().textContent).not.toContain('Moved to');
    expect(status.element().textContent).not.toContain('cancelled');
    await mouse.to(10);
    await mouse.up();
    await shift(false);
    await expect.element(status).toMatchTextContent(`Moved to (184, ${g},`);
    // Nothing was left behind: a click still picks, at the pixel it would without a pan.
    await click(-10); // column 14
    expect(colors()[1]).toMatch(/^rgb 75, /);
  });

  for (const how of ['Escape', 'pointercancel', 'lostpointercapture'] as const) {
    test(`${how} puts the pick back as it was before the drag`, async () => {
      const { colors, selects, mouse, shift, status, nextPointerId, synthetic } = await opened(
        await fixtureFile('opaque'),
        [0],
      );
      const before = colors();
      await userEvent.selectOptions(selects()[0]!, '2');
      await expect.poll(() => selects()[0]!.selectedIndex).toBe(2);
      await shift(true);
      const id = nextPointerId();
      await mouse.down(0);
      await mouse.to(10);
      await expect.poll(() => colors()[0]).toMatch(/^rgb 184, /);
      expect(selects()[0]!.selectedIndex).toBe(0);
      if (how === 'Escape') await userEvent.keyboard('{Escape}');
      else synthetic(how, await id, 10);
      await expect.element(status).toHaveTextContent('Move cancelled.');
      expect(colors()).toEqual(before);
      expect(selects()[0]!.selectedIndex).toBe(2); // the hand-chosen ink is back too
      await mouse.to(15);
      await mouse.up();
      await shift(false);
      await wait(300);
      expect(colors()).toEqual(before); // the rest of the gesture does nothing
      await expect.element(status).toHaveTextContent('Move cancelled.');
    });
  }

  test('a drag without Shift pans, leaves the picks alone and ignores other pointers', async () => {
    const { colors, mouse, status, click, nextPointerId, synthetic, acceptSyntheticCapture } =
      await opened(await fixtureFile('opaque'), [0]);
    acceptSyntheticCapture();
    const before = colors();
    const id = nextPointerId();
    await mouse.down(0);
    const other = (await id) + 100;
    synthetic('pointerdown', other, -15); // a second pointer can't start a pan of its own
    synthetic('pointercancel', other, -15); // nor end this one
    synthetic('lostpointercapture', other, -15);
    await mouse.to(10); // pans the image 10 px right
    await mouse.up();
    await wait(300);
    expect(colors()).toEqual(before);
    expect(status.element().textContent).not.toContain('Moved to');
    await click(-10); // column 24 − 10 − 10 = 4, so the pan happened
    expect(colors()[1]).toMatch(/^rgb 21, /);
  });

  // ── The colorized ink dropdown ─────────────────────────────────────────────────────────────────

  /** Customizable select, as `PickList`'s `@supports` gate decides (Chromium, WebKit; not Firefox). */
  const customized =
    CSS.supports('appearance', 'base-select') && CSS.supports('selector(::picker(select))');

  const visible = (e: Element | null) => {
    const box = e?.getBoundingClientRect();
    return !!box && box.width > 0 && box.height > 0;
  };

  test("each ink option carries its ink's color", async () => {
    const { screen, selects } = await opened(await fixtureFile('opaque'), [0]);
    const select = selects()[0]!;
    const swatches = [...select.options].map((o) => o.querySelector<HTMLElement>('.option-swatch'));
    expect(swatches).toHaveLength(8);
    for (const swatch of swatches) expect(swatch?.style.background).toMatch(/^rgb\(/);
    // The chosen option's swatch is the ink swatch shown in the pick row.
    const ink = screen.container.querySelector<HTMLElement>('.pick .swatch.ink')!;
    expect(swatches[select.selectedIndex]!.style.background).toBe(ink.style.background);
    // Choosing an option makes its swatch's color (read before choosing) the ink.
    const third = swatches[3]!.style.background;
    expect(third).not.toBe(ink.style.background);
    await userEvent.selectOptions(select, '3');
    await expect.poll(() => ink.style.background).toBe(third);
  });

  test.runIf(customized)('customized select: the swatches are shown, closed and open', async () => {
    const { screen, selects } = await opened(await fixtureFile('opaque'), [0]);
    const select = selects()[0]!;
    expect(getComputedStyle(select).appearance).toBe('base-select');
    const ink = screen.container.querySelector<HTMLElement>('.pick .swatch.ink')!;
    const closed = select.querySelector('selectedcontent .option-swatch');
    expect(visible(closed)).toBe(true);
    expect(getComputedStyle(closed!).backgroundColor).toBe(getComputedStyle(ink).backgroundColor);
    const swatches = [...select.options].map((o) => o.querySelector('.option-swatch'));
    expect(swatches.some(visible)).toBe(false); // closed: the list is not shown
    await userEvent.click(select);
    await expect.poll(() => swatches.every(visible)).toBe(true);
    await userEvent.keyboard('{Escape}');
    await expect.poll(() => swatches.some(visible)).toBe(false);
  });

  test.runIf(!customized)(
    'native select fallback: full ink names, and it still chooses',
    async () => {
      const { screen, selects } = await opened(await fixtureFile('opaque'), [0]);
      const select = selects()[0]!;
      expect(getComputedStyle(select).appearance).not.toBe('base-select');
      const labels = [...select.options].map((o) => o.label);
      expect(labels).toHaveLength(8);
      for (const label of labels) expect(label).toMatch(/^\S.* · ΔE \d+\.\d$/);
      const ink = screen.container.querySelector<HTMLElement>('.pick .swatch.ink')!;
      await userEvent.selectOptions(select, '3');
      await expect.poll(() => ink.title).toBe(labels[3]!.replace(/ · ΔE .*$/, ''));
    },
  );

  // ── The material color ─────────────────────────────────────────────────────────────────────────
  // At 1:1, offsets from the canvas center: −18 → column 6 (opaque red), −6 → 18 (translucent
  // red), +6 → 30 (transparent), +18 → 42 (opaque black).

  /** 48 × 32 in four 12-column bands: opaque red, translucent red, transparent, opaque black. */
  async function materialFile(): Promise<File> {
    const canvas = new OffscreenCanvas(48, 32);
    const context = canvas.getContext('2d')!;
    context.fillStyle = 'rgb(200 40 40)';
    context.fillRect(0, 0, 12, 32);
    context.fillStyle = 'rgb(200 40 40 / 0.4)';
    context.fillRect(12, 0, 12, 32);
    context.fillStyle = 'rgb(0 0 0)';
    context.fillRect(36, 0, 12, 32);
    return new File([await canvas.convertToBlob({ type: 'image/png' })], 'bands.png');
  }

  const BANDS = [-18, -6, 6, 18];
  /** The Original's and the preview's frames. */
  const frames = (screen: { container: HTMLElement }) => [
    ...screen.container.querySelectorAll<HTMLElement>('.views .frame'),
  ];
  /** The "Not printed" rows' labels, in order. */
  const rangeRows = (screen: { container: HTMLElement }) =>
    [...screen.container.querySelectorAll('.range-color')].map((e) => e.textContent!.trim());
  const materialInput = (screen: { container: HTMLElement }) =>
    screen.container.querySelector<HTMLInputElement>('[data-testid=material]')!;
  /** The preview's frame: the material is behind it, not behind the Original. */
  const previewFrame = (screen: { container: HTMLElement }) => frames(screen)[1]!;

  /** The RGBA pixel at (x, y) of the downloaded PNG. */
  async function downloadedPixel(
    screen: { getByTestId: (id: string) => { click(): Promise<void> } },
    x: number,
    y: number,
  ) {
    const download = captureDownload();
    await screen.getByTestId('download').click();
    const bitmap = await createImageBitmap((await download).blob);
    const context = new OffscreenCanvas(bitmap.width, bitmap.height).getContext('2d')!;
    context.drawImage(bitmap, 0, 0);
    return [...context.getImageData(x, y, 1, 1).data];
  }

  test('no material at first; a chosen one is behind the preview only', async () => {
    const { screen, click, status } = await opened(await materialFile(), []);
    const [original, preview] = frames(screen) as [HTMLElement, HTMLElement];
    const download = screen.getByTestId('download');
    const white = screen.getByRole('button', { name: 'White' });
    // "None": a checkerboard swatch and frames, White and Black available, nothing unprinted.
    expect(screen.container.querySelector('.material-swatch.none')).not.toBeNull();
    for (const frame of [original, preview]) {
      expect(getComputedStyle(frame).backgroundImage).toMatch(/conic-gradient/);
    }
    await expect.element(white).toBeEnabled();
    await expect.element(screen.getByRole('button', { name: 'Black' })).toBeEnabled();
    await expect.element(screen.getByRole('button', { name: 'Reset' })).toBeDisabled();
    expect(rangeRows(screen)).toEqual([]);
    // No picks, nothing printed: an empty preview without a placeholder, nothing to download.
    expect(preview.classList.contains('empty')).toBe(true);
    expect(preview.querySelector('.placeholder')).toBeNull();
    await expect.element(download).toBeDisabled();

    await userEvent.fill(screen.getByTestId('material'), '#102030');
    await expect.element(status).toHaveTextContent('Material #102030.');
    // The material is behind the preview only (its frame, not its canvas); the Original keeps
    // the checkerboard.
    expect(getComputedStyle(preview).backgroundColor).toBe('rgb(16, 32, 48)');
    expect(getComputedStyle(preview).backgroundImage).toBe('none');
    expect(getComputedStyle(original).backgroundImage).toMatch(/conic-gradient/);
    // Still nothing printed, so only the material shows.
    expect(preview.classList.contains('empty')).toBe(true);
    await expect.element(screen.getByText('No picks yet: nothing is printed')).toBeVisible();
    await expect.element(download).toBeDisabled();
    expect(rangeRows(screen)).toEqual(['Material color · #102030']);

    // The first pick prints; the material's own color (the transparent band) stays unprinted.
    await click(-18);
    await expect.element(screen.getByText(/^Printed with 1 ink/)).toBeVisible();
    await expect.poll(() => preview.classList.contains('empty')).toBe(false);
    await expect.element(download).toBeEnabled();
    expect(await downloadedPixel(screen, 30, 16)).toEqual([0, 0, 0, 0]);
    expect((await downloadedPixel(screen, 6, 16))[3]).toBe(255);

    await white.click();
    await expect.element(status).toHaveTextContent('Material #ffffff.');
    expect(materialInput(screen).value).toBe('#ffffff');
    expect(getComputedStyle(preview).backgroundColor).toBe('rgb(255, 255, 255)');
    expect(getComputedStyle(original).backgroundImage).toMatch(/conic-gradient/);
    await expect.element(white).toBeDisabled();
    expect(rangeRows(screen)).toEqual(['Material color · #ffffff']);
    expect(await downloadedPixel(screen, 30, 16)).toEqual([0, 0, 0, 0]);

    // Without picks again, nothing is printed again.
    await screen.getByRole('button', { name: 'Clear all' }).click();
    await expect.element(screen.getByText('No picks yet: nothing is printed')).toBeVisible();
    await expect.poll(() => preview.classList.contains('empty')).toBe(true);
    await expect.element(download).toBeDisabled();
  });

  test('a material change re-suggests only the picks whose color changed', async () => {
    const { screen, colors, selects, status } = await opened(await materialFile(), BANDS);
    expect(colors()).toHaveLength(4);
    // A hand-chosen ink on the opaque red pick.
    await userEvent.selectOptions(selects()[0]!, '3');
    await expect.poll(() => selects()[0]!.selectedIndex).toBe(3);
    const ink = () => selects().map((s) => s.selectedOptions[0]!.text);
    const before = ink();
    expect(before[2]).toMatch(/^Pure White/); // transparent on white

    await userEvent.fill(screen.getByTestId('material'), '#000000');
    await expect.element(status).toHaveTextContent('Material #000000 · 2 picks re-suggested.');
    const after = ink();
    expect(selects()[0]!.selectedIndex).toBe(3); // opaque: the hand choice is kept
    expect(after[0]).toBe(before[0]);
    expect(selects()[1]!.selectedIndex).toBe(0); // translucent: re-suggested
    expect(after[1]).not.toBe(before[1]);
    expect(after[2]).toMatch(/^Pure Black/); // transparent: now the material, black
    expect(after[3]).toBe(before[3]); // opaque black: unchanged
    // The transparent and the black pick now share (0, 0, 0); both stay.
    expect(colors()).toHaveLength(4);
    await expect.element(screen.getByText(/^Printed with 4 inks/)).toBeVisible();
    // The transparent band is the material, which is left unprinted.
    expect(await downloadedPixel(screen, 30, 16)).toEqual([0, 0, 0, 0]);
    // Without that entry it prints with the ink nearest the material.
    await screen.getByRole('button', { name: /^Print .* again$/ }).click();
    await expect.poll(() => rangeRows(screen)).toEqual([]);
    expect(await downloadedPixel(screen, 30, 16)).toEqual([0, 0, 0, 255]);

    // Back to white: nothing was removed, the hand ink is still there.
    await screen.getByRole('button', { name: 'White' }).click();
    await expect.element(status).toHaveTextContent('Material #ffffff · 2 picks re-suggested.');
    expect(colors()).toHaveLength(4);
    expect(selects()[0]!.selectedIndex).toBe(3);
    expect(ink()[2]).toMatch(/^Pure White/);
  });

  test('a burst of picker changes applies only the newest one', async () => {
    const { screen, status } = await opened(await materialFile(), BANDS);
    const posted = vi.spyOn(Worker.prototype, 'postMessage');
    const input = materialInput(screen);
    for (const color of ['#101010', '#202020', '#303030', '#404040', '#000000']) {
      input.value = color;
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await expect.element(status).toHaveTextContent('Material #000000 · 2 picks re-suggested.');
    const rematches = posted.mock.calls.filter(
      ([message]) => (message as { path?: string[] } | null)?.path?.[0] === 'rematch',
    );
    expect(rematches).toHaveLength(1);
  });

  test('export writes the material; import sets it (white if the file has none)', async () => {
    const { screen, status } = await opened(await materialFile(), [-18]);
    await userEvent.fill(screen.getByTestId('material'), '#000000');
    await expect.element(status).toHaveTextContent('Material #000000.');
    const download = captureDownload();
    await screen.getByRole('button', { name: 'Export picks' }).click();
    const exported = await download;
    expect(exported.name).toBe('bands.palettes.toml');
    expect(await exported.blob.text()).toContain('\nmaterial = [0, 0, 0]\n');

    const configInput = screen.container.querySelector<HTMLInputElement>('input[accept^=".toml"]')!;
    const config = (material: string) =>
      new File(
        [
          material +
            '[[palette]]\nsize = 1\npicks = [\n' +
            '  { rgba = [0, 0, 0, 0], ink = "Pantone 285" },\n]\n',
        ],
        'one.palettes.toml',
      );
    await userEvent.upload(page.elementLocator(configInput), config('material = [16, 32, 48]\n\n'));
    await expect
      .element(status)
      .toHaveTextContent('Imported 1 pick (size 1) from one.palettes.toml, on material #102030.');
    expect(materialInput(screen).value).toBe('#102030');
    expect(getComputedStyle(previewFrame(screen)).backgroundColor).toBe('rgb(16, 32, 48)');
    // The transparent pick is matched on the file's material.
    const matching = () =>
      screen.container.querySelector<HTMLElement>('.pick .swatch:not(.checker):not(.ink)')!.title;
    expect(matching()).toBe('On the material: #102030');

    await userEvent.upload(page.elementLocator(configInput), config(''));
    await expect
      .element(status)
      .toHaveTextContent('Imported 1 pick (size 1) from one.palettes.toml, on material #ffffff.');
    expect(materialInput(screen).value).toBe('#ffffff');
    expect(matching()).toBe('On white: #ffffff');
  });

  test('a move cancelled after a material change puts the pick back matched on the new one', async () => {
    const { screen, colors, selects, mouse, shift, status } = await opened(
      await materialFile(),
      [-6],
    );
    await userEvent.selectOptions(selects()[0]!, '2');
    await expect.poll(() => selects()[0]!.selectedIndex).toBe(2);
    const before = colors();
    const [, r, g, b, a] = /^rgb (\d+), (\d+), (\d+), alpha (\d+)$/.exec(before[0]!)!.map(Number);
    await shift(true);
    await mouse.down(-6);
    await mouse.to(-18); // the opaque band: another color
    await expect.poll(() => colors()[0]).not.toBe(before[0]);
    // The material changes while the pick is still held (rare, but possible from the keyboard).
    const input = materialInput(screen);
    input.value = '#000000';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await expect.element(status).toMatchTextContent('Material #000000');
    await userEvent.keyboard('{Escape}');
    await expect.element(status).toHaveTextContent('Move cancelled.');
    await mouse.up();
    await shift(false);
    expect(colors()).toEqual(before); // the stored pixel is back…
    // …matched on black, so its color changed and the ink is the nearest again.
    const onBlack = [r!, g!, b!].map((c) => Math.floor((a! * c) / 255));
    const title = screen.container.querySelector<HTMLElement>(
      '.pick .swatch:not(.checker):not(.ink)',
    )!.title;
    expect(title).toBe(
      `On the material: #${onBlack.map((c) => c.toString(16).padStart(2, '0')).join('')}`,
    );
    expect(selects()[0]!.selectedIndex).toBe(0);
  });

  test('material changes keep their order with an import between them', async () => {
    const { screen, status } = await opened(await materialFile(), [-6]);
    // Hold the worker's `rematch` messages, so the first material change keeps the queue busy.
    const send = Worker.prototype.postMessage;
    const held: (() => void)[] = [];
    vi.spyOn(Worker.prototype, 'postMessage').mockImplementation(function (
      this: Worker,
      ...args: Parameters<Worker['postMessage']>
    ) {
      const call = () => send.apply(this, args);
      if ((args[0] as { path?: string[] } | null)?.path?.[0] === 'rematch' && held.length === 0) {
        held.push(call);
      } else {
        call();
      }
    });
    const input = materialInput(screen);
    const choose = (color: string) => {
      input.value = color;
      input.dispatchEvent(new Event('input', { bubbles: true }));
    };
    choose('#101010'); // X: starts, and waits for the held re-match
    await expect.poll(() => held.length).toBe(1);
    choose('#202020'); // A: queued behind X
    const configInput = screen.container.querySelector<HTMLInputElement>('input[accept^=".toml"]')!;
    await userEvent.upload(
      page.elementLocator(configInput),
      new File(
        [
          'material = [16, 32, 48]\n\n[[palette]]\nsize = 1\npicks = [\n' +
            '  { rgba = [0, 0, 0, 0], ink = "Pantone 285" },\n]\n',
        ],
        'one.palettes.toml',
      ),
    ); // the import: queued behind A
    choose('#000000'); // B: the last action, so it must apply last
    held[0]!();

    await expect.element(status).toMatchTextContent('Material #000000');
    expect(getComputedStyle(previewFrame(screen)).backgroundColor).toBe('rgb(0, 0, 0)');
    expect(input.value).toBe('#000000');
    // The imported pick, re-matched on B.
    const title = screen.container.querySelector<HTMLElement>(
      '.pick .swatch:not(.checker):not(.ink)',
    )!.title;
    expect(title).toBe('On the material: #000000');
  });

  // ── Colors left unprinted ──────────────────────────────────────────────────────────────────────

  test('a chosen material adds one "Not printed" entry that follows it and keeps its ΔE', async () => {
    const { screen } = await opened(await materialFile(), []);
    const slider = () => screen.container.querySelector<HTMLInputElement>('.range-delta input')!;
    await screen.getByRole('button', { name: 'Black' }).click();
    await expect.poll(() => rangeRows(screen)).toEqual(['Material color · #000000']);
    expect(slider().value).toBe('10');
    // A hand-set ΔE is kept when the material changes; still one entry.
    slider().value = '25';
    slider().dispatchEvent(new Event('input', { bubbles: true }));
    await userEvent.fill(screen.getByTestId('material'), '#102030');
    await expect.poll(() => rangeRows(screen)).toEqual(['Material color · #102030']);
    expect(slider().value).toBe('25');
    // Removed, then the next choice adds it again with the default ΔE.
    await screen.getByRole('button', { name: /^Print .* again$/ }).click();
    await expect.poll(() => rangeRows(screen)).toEqual([]);
    await screen.getByRole('button', { name: 'Black' }).click();
    await expect.poll(() => rangeRows(screen)).toEqual(['Material color · #000000']);
    expect(slider().value).toBe('10');
  });

  test('+ then a click leaves a color unprinted, marked with a square', async () => {
    const { screen, at, status } = await opened(await materialFile(), [-18]); // a pick: a circle
    const overlay = screen.container.querySelector<HTMLCanvasElement>(
      `${ORIGINAL} ~ canvas.overlay`,
    )!;
    const rects = vi.spyOn(CanvasRenderingContext2D.prototype, 'rect');
    const arcs = vi.spyOn(CanvasRenderingContext2D.prototype, 'arc');
    const add = screen.getByRole('button', { name: 'Leave a color unprinted' });
    await add.click();
    await expect.element(add).toHaveAttribute('aria-pressed', 'true');
    await expect
      .element(
        screen.getByText('Original — click a color to leave it unprinted (the material shows)'),
      )
      .toBeVisible();
    await userEvent.click(page.elementLocator(overlay.previousElementSibling!), {
      position: at(-15), // column 9: the opaque red band
    });
    await expect.element(status).toMatchTextContent('Left to the material: (200, 40, 40)');
    await expect.element(add).toHaveAttribute('aria-pressed', 'false');
    expect(rangeRows(screen)).toEqual(['rgb 200, 40, 40']);
    await expect.element(screen.getByText(/· 1 color left to the material/)).toBeVisible();
    // A square at the clicked pixel (11 px, centered); the pick keeps its circle.
    const square = rects.mock.calls.find(
      ([x, y, w]) =>
        w === 11 && Math.abs(x + 5.5 - at(-15).x) <= 1 && Math.abs(y + 5.5 - at(-15).y) <= 1,
    );
    expect(square, JSON.stringify(rects.mock.calls.slice(-3))).toBeDefined();
    expect(arcs.mock.calls.some(([x]) => Math.abs(x - at(-18).x) <= 1)).toBe(true);
    // Transparent in the result; × prints it again.
    expect(await downloadedPixel(screen, 6, 16)).toEqual([0, 0, 0, 0]);
    await screen.getByRole('button', { name: 'Print rgb 200, 40, 40 again' }).click();
    await expect.poll(() => rangeRows(screen)).toEqual([]);
    expect((await downloadedPixel(screen, 6, 16))[3]).toBe(255);
  });

  test('a new image keeps the material entry and clears the clicked ones', async () => {
    const { screen, at, status } = await opened(await materialFile(), []);
    await screen.getByRole('button', { name: 'Black' }).click();
    await expect.poll(() => rangeRows(screen)).toHaveLength(1);
    await screen.getByRole('button', { name: 'Leave a color unprinted' }).click();
    await userEvent.click(page.elementLocator(screen.container.querySelector(ORIGINAL)!), {
      position: at(-15),
    });
    await expect.element(status).toMatchTextContent('Left to the material');
    expect(rangeRows(screen)).toEqual(['Material color · #000000', 'rgb 200, 40, 40']);
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await materialFile());
    await expect.element(status).toHaveTextContent('Click a color in the original to pick it.');
    expect(rangeRows(screen)).toEqual(['Material color · #000000']);
  });

  test('unprinted colors go out with an export and come back with an import', async () => {
    const { screen, status } = await opened(await materialFile(), []);
    const configInput = screen.container.querySelector<HTMLInputElement>('input[accept^=".toml"]')!;
    const entries =
      'unprinted = [\n' +
      '  { material = true, delta_e = 25 },\n' +
      '  { rgba = [200, 40, 40, 255], delta_e = 10 },\n]\n';
    const config = (unprinted: string) =>
      'material = [255, 255, 255]\n' +
      unprinted +
      '\n[[palette]]\nsize = 1\npicks = [\n' +
      '  { rgba = [0, 0, 0, 255], ink = "Pure Black (non-palette)" },\n]\n';
    expect(screen.container.querySelector('.material-swatch.none')).not.toBeNull();
    await userEvent.upload(
      page.elementLocator(configInput),
      new File([config(entries)], 'two.palettes.toml'),
    );
    await expect
      .element(status)
      .toHaveTextContent(
        'Imported 1 pick (size 1) from two.palettes.toml, 2 colors left unprinted.',
      );
    // White, but the file lists the material's own color, so the material counts as chosen.
    expect(screen.container.querySelector('.material-swatch.none')).toBeNull();
    expect(rangeRows(screen)).toEqual(['Material color · #ffffff', 'rgb 200, 40, 40']);
    const sliders = () =>
      [...screen.container.querySelectorAll<HTMLInputElement>('.range-delta input')].map(
        (e) => e.value,
      );
    expect(sliders()).toEqual(['25', '10']);
    // Both are transparent in the result: the transparent band (the material) and the red band.
    expect(await downloadedPixel(screen, 30, 16)).toEqual([0, 0, 0, 0]);
    expect(await downloadedPixel(screen, 6, 16)).toEqual([0, 0, 0, 0]);

    // Exported as they are.
    const download = captureDownload();
    await screen.getByRole('button', { name: 'Export picks' }).click();
    expect(await (await download).blob.text()).toContain(
      `\nmaterial = [255, 255, 255]\n${entries}`,
    );

    // A file without unprinted colors leaves none (the file is the whole state).
    await userEvent.upload(
      page.elementLocator(configInput),
      new File([config('')], 'none.palettes.toml'),
    );
    await expect
      .element(status)
      .toHaveTextContent('Imported 1 pick (size 1) from none.palettes.toml.');
    expect(rangeRows(screen)).toEqual([]);
    // White with no material entry is "none" again: swatch and both frames.
    expect(screen.container.querySelector('.material-swatch.none')).not.toBeNull();
    for (const frame of frames(screen)) {
      expect(getComputedStyle(frame).backgroundImage).toMatch(/conic-gradient/);
    }
  });

  test('removing an entry during a material change keeps it removed', async () => {
    const { screen, status } = await opened(await materialFile(), [-18]); // a pick: a re-match
    await screen.getByRole('button', { name: 'Black' }).click();
    await expect.poll(() => rangeRows(screen)).toEqual(['Material color · #000000']);
    // Hold the next re-match, so the material change below waits in the queue.
    const send = Worker.prototype.postMessage;
    const held: (() => void)[] = [];
    vi.spyOn(Worker.prototype, 'postMessage').mockImplementation(function (
      this: Worker,
      ...args: Parameters<Worker['postMessage']>
    ) {
      const call = () => send.apply(this, args);
      if ((args[0] as { path?: string[] } | null)?.path?.[0] === 'rematch' && held.length === 0) {
        held.push(call);
      } else {
        call();
      }
    });
    const input = materialInput(screen);
    input.value = '#102030';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await expect.poll(() => held.length).toBe(1);
    // The later action: remove the material's entry while the change still waits.
    await screen.getByRole('button', { name: /^Print .* again$/ }).click();
    held[0]!();
    await expect.element(status).toMatchTextContent('Material #102030');
    await expect.poll(() => rangeRows(screen)).toEqual([]);
  });

  test('an imported ΔE above 40 stays as it is and can be edited', async () => {
    const { screen, status } = await opened(await materialFile(), []);
    const configInput = screen.container.querySelector<HTMLInputElement>('input[accept^=".toml"]')!;
    const config =
      'material = [0, 0, 0]\nunprinted = [\n' +
      '  { rgba = [200, 40, 40, 255], delta_e = 75 },\n' +
      '  { rgba = [0, 0, 0, 255], delta_e = 30 },\n]\n\n' +
      '[[palette]]\nsize = 1\npicks = [\n' +
      '  { rgba = [0, 0, 0, 255], ink = "Pure Black (non-palette)" },\n]\n';
    await userEvent.upload(
      page.elementLocator(configInput),
      new File([config], 'wide.palettes.toml'),
    );
    await expect.element(status).toMatchTextContent('2 colors left unprinted');
    const sliders = () => [
      ...screen.container.querySelectorAll<HTMLInputElement>('.range-delta input'),
    ];
    const outputs = () =>
      [...screen.container.querySelectorAll('.range-delta output')].map((e) => e.textContent);
    // The value above 40 is kept, and the slider and the number agree; the other keeps 0–40.
    expect(sliders().map((e) => [e.value, e.max])).toEqual([
      ['75', '100'],
      ['30', '40'],
    ]);
    expect(outputs()).toEqual(['75', '30']);
    // Edited, then exported as edited.
    sliders()[0]!.value = '60';
    sliders()[0]!.dispatchEvent(new Event('input', { bubbles: true }));
    await expect.poll(outputs).toEqual(['60', '30']);
    const download = captureDownload();
    await screen.getByRole('button', { name: 'Export picks' }).click();
    const text = await (await download).blob.text();
    expect(text).toContain('{ rgba = [200, 40, 40, 255], delta_e = 60 }');
    expect(text).toContain('{ rgba = [0, 0, 0, 255], delta_e = 30 }');
  });

  test('Reset goes back to no material and keeps the clicked unprinted colors', async () => {
    // A translucent pick: its color changes with the material.
    const { screen, at, status } = await opened(await materialFile(), [-6]);
    const reset = screen.getByRole('button', { name: 'Reset' });
    await screen.getByRole('button', { name: 'Black' }).click();
    await expect.element(status).toHaveTextContent('Material #000000 · 1 pick re-suggested.');
    await screen.getByRole('button', { name: 'Leave a color unprinted' }).click();
    await userEvent.click(page.elementLocator(screen.container.querySelector(ORIGINAL)!), {
      position: at(-15),
    });
    await expect.element(status).toMatchTextContent('Left to the material');
    expect(rangeRows(screen)).toEqual(['Material color · #000000', 'rgb 200, 40, 40']);
    await expect.element(reset).toBeEnabled();

    await reset.click();
    await expect.element(status).toHaveTextContent('Material reset · 1 pick re-suggested.');
    // "None" again: swatch, frames, the material's own entry gone (the clicked one stays).
    expect(screen.container.querySelector('.material-swatch.none')).not.toBeNull();
    for (const frame of frames(screen)) {
      expect(getComputedStyle(frame).backgroundImage).toMatch(/conic-gradient/);
    }
    expect(rangeRows(screen)).toEqual(['rgb 200, 40, 40']);
    expect(materialInput(screen).value).toBe('#ffffff');
    await expect.element(reset).toBeDisabled();
    // The pick is matched on white again.
    const matching = screen.container.querySelector<HTMLElement>(
      '.pick .swatch:not(.checker):not(.ink)',
    )!;
    expect(matching.title).toMatch(/^On white: #/);
  });
});
