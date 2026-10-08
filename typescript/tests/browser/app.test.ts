// End-to-end smoke test (plan step 5): the real app in each engine — open → pick → live recolor →
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

  // ── Moving picks (`.agents/repositioning`) ──────────────────────────────────────────────────
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
    await expect
      .element(screen.getByText('No picks yet: the image as printed on white'))
      .toBeVisible();
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

  test('D2: a held Shift-drag moves the pick live, before the release', async () => {
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

  test('D1, C1 a: a hand-chosen ink stays within its color and is replaced by a new one', async () => {
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
    expect(selects()[0]!.selectedIndex).toBe(3); // C1 a: the hand choice is kept

    await mouse.down(-3);
    await mouse.to(6); // column 30: the other color
    await mouse.up();
    await shift(false);
    await expect.element(status).toMatchTextContent('Moved to (40, 40, 200)');
    expect(colors()).toEqual(['rgb 40, 40, 200']);
    expect(selects()[0]!.selectedIndex).toBe(0); // D1: the nearest ink for the new color
    expect(selects()[0]!.selectedOptions[0]!.text).not.toBe(hand);
    await expect.element(screen.getByText('Printed with 1 ink')).toBeVisible();
  });

  test('D3, F2 a: a drop on a picked color leaves the pick on the last free pixel', async () => {
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

  test('D4: away from the circles, a Shift-drag pans and a Shift-click picks', async () => {
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
    test(`F1 a: ${how} puts the pick back as it was before the drag`, async () => {
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

  // ── The colorized ink dropdown (`.agents/colorized-drobox`) ─────────────────────────────────────

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
});
