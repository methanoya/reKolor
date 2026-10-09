// The LogRocket events the app reports, in a real browser. `vi.mock` with `spy: true` keeps the
// real functions of `src/lib/logrocket.ts` and records their calls; on this test page (not the
// published site) they send nothing, so the calls are all there is to check.
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import App from '../../src/App.svelte';
import '../../src/app.css';
import { EngineClient } from '../../src/lib/client';
import { track } from '../../src/lib/logrocket';
import { fixtureFile } from './fixtures';

vi.mock('../../src/lib/logrocket', { spy: true });

// Each test looks at its own calls only; `vi.spyOn` replacements are undone after each test.
beforeEach(() => vi.mocked(track).mockClear());
afterEach(() => vi.restoreAllMocks());

/** The "Image opened" event's properties, once it has been reported. `expect.poll` below retries it
 * (for up to 15 s: opening and counting take a moment, more on a cold start). */
const opened = () => vi.mocked(track).mock.calls.find(([event]) => event === 'Image opened')?.[1];

describe('LogRocket events', () => {
  test('an opened image is reported by format, size and color count, never by name', async () => {
    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    // A PNG without a type, whose name ends in something that isn't an image format.
    const file = new File([await fixtureFile('opaque')], 'Client Secret.patient-id');
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), file);

    await expect.poll(opened, { timeout: 15_000 }).toBeDefined();
    expect(opened()).toEqual({
      format: 'other',
      width: 48,
      height: 32,
      colors: expect.any(Number),
    });
  });

  test('the event keeps its color count when the material changes while counting', async () => {
    // Holds back every color count until the test releases it, so the counts can finish in the
    // order that used to lose the number: the open's count first, while the count for a new
    // material is still running. `EngineClient.prototype.call` runs every engine call; the
    // replacement passes the call a stand-in for the engine whose `colorCount` waits for its turn.
    // `releases[i]` lets the i-th count go on.
    const releases: (() => void)[] = [];
    type Call = EngineClient['call'];
    type Api = Parameters<Parameters<Call>[0]>[0];
    const realCall: Call = EngineClient.prototype.call;
    const holdCounts = (api: Api): Api =>
      new Proxy(api, {
        get(target, key) {
          if (key !== 'colorCount') return Reflect.get(target, key) as unknown;
          return async (...args: Parameters<Api['colorCount']>) => {
            await new Promise<void>((resolve) => releases.push(resolve));
            return target.colorCount(...args);
          };
        },
      });
    vi.spyOn(EngineClient.prototype, 'call').mockImplementation(function (this: EngineClient, f) {
      return realCall.call(this, (api) => f(holdCounts(api)));
    });

    const screen = await render(App);
    await expect.element(screen.getByText(/Engine ready/)).toBeVisible();
    const input = screen.container.querySelector<HTMLInputElement>('#file')!;
    await userEvent.upload(page.elementLocator(input), await fixtureFile('opaque'));
    // The open's count is held.
    await expect.poll(() => releases.length, { timeout: 15_000 }).toBe(1);
    // A new material starts a second count, held too.
    await screen.getByRole('button', { name: 'Black' }).click();
    await expect.poll(() => releases.length, { timeout: 15_000 }).toBe(2);

    // The open's count finishes first, while the second is still running (the toolbar is still
    // counting): the event carries the open's count.
    releases[0]!();
    await expect.poll(opened, { timeout: 15_000 }).toBeDefined();
    expect(opened()).toEqual({ format: 'png', width: 48, height: 32, colors: expect.any(Number) });
    await expect.element(screen.getByTestId('file-info')).toMatchTextContent('counting colors…');

    // Then the second count finishes and shows in the toolbar.
    releases[1]!();
    await expect.element(screen.getByTestId('file-info')).toMatchTextContent(/\d colors/);
  });
});
