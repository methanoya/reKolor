// Unit tests for `logrocket.ts`, with LogRocket replaced by a fake (`vi.mock`), so nothing is loaded
// or sent. `vi.stubGlobal` sets the page's host (`location`); `vi.stubEnv` sets `VITE_RELEASE`.
import { afterEach, describe, expect, test, vi } from 'vitest';

// The fake LogRocket. `vi.hoisted` runs before the module mock is set up (Vitest moves `vi.mock`
// to the top of the file), so the mock can use it.
const fake = vi.hoisted(() => ({ init: vi.fn(), track: vi.fn(), captureMessage: vi.fn() }));
vi.mock('logrocket', () => ({ default: fake }));

/** A fresh copy of the module (its recording state starts over), on a page served from `host`. */
async function load(host: string) {
  vi.stubGlobal('location', { hostname: host });
  vi.resetModules();
  return import('./logrocket');
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
  vi.clearAllMocks();
});

describe('LogRocket', () => {
  test('anywhere but the published site, nothing starts and nothing is sent', async () => {
    for (const host of ['localhost', '127.0.0.1', 'example.github.io']) {
      const lr = await load(host);
      lr.startRecording();
      lr.track('Pick added', { picks: 1 });
      lr.reportError('decodeFailed');
      await vi.dynamicImportSettled();
    }
    expect(fake.init).not.toHaveBeenCalled();
    expect(fake.track).not.toHaveBeenCalled();
    expect(fake.captureMessage).not.toHaveBeenCalled();
  });

  test('on the published site it starts once: app ID, release, no IPs, no network', async () => {
    vi.stubEnv('VITE_RELEASE', 'abc1234');
    const lr = await load('methanoya.github.io');
    lr.startRecording();
    lr.startRecording();
    await vi.dynamicImportSettled();
    expect(fake.init).toHaveBeenCalledTimes(1);
    expect(fake.init).toHaveBeenCalledWith('gzqrb0/rekolor', {
      release: 'abc1234',
      shouldCaptureIP: false,
      network: { isEnabled: false },
    });
  });

  test('without VITE_RELEASE (a local build), the release is "local"', async () => {
    vi.stubEnv('VITE_RELEASE', undefined);
    const lr = await load('methanoya.github.io');
    lr.startRecording();
    await vi.dynamicImportSettled();
    expect(fake.init).toHaveBeenCalledWith(
      'gzqrb0/rekolor',
      expect.objectContaining({ release: 'local' }),
    );
  });

  test('an image format comes from a fixed list, never from the file name', async () => {
    const { imageFormat } = await load('localhost');
    expect(imageFormat({ name: 'photo.png', type: 'image/png' })).toBe('png');
    expect(imageFormat({ name: 'icon.ico', type: 'image/vnd.microsoft.icon' })).toBe('ico');
    // Without a type (the browser doesn't know EXR, say), the extension, if it is in the list.
    expect(imageFormat({ name: 'Photo.JPG', type: '' })).toBe('jpeg');
    expect(imageFormat({ name: 'render.exr', type: '' })).toBe('exr');
    // Anything else is `other`: no part of a name gets through.
    expect(imageFormat({ name: 'scan.patient-id', type: '' })).toBe('other');
    expect(imageFormat({ name: 'Client Secret', type: '' })).toBe('other');
    expect(imageFormat({ name: 'x.constructor', type: '' })).toBe('other');
    expect(imageFormat({ name: 'x.png', type: 'image/x-client-secret' })).toBe('other');
  });

  test('calls made while LogRocket loads are sent, in order, once it has started', async () => {
    const lr = await load('methanoya.github.io');
    lr.startRecording();
    lr.track('Image opened', { format: 'image/png', width: 2, height: 3 });
    lr.reportError('invalidConfig');
    // Still loading: nothing sent yet.
    expect(fake.track).not.toHaveBeenCalled();
    await vi.dynamicImportSettled();
    lr.track('Pick added', { picks: 1 });
    expect(fake.track.mock.calls).toEqual([
      ['Image opened', { format: 'image/png', width: 2, height: 3 }],
      ['Pick added', { picks: 1 }],
    ]);
    // An error goes by its kind only, never the message the user saw.
    expect(fake.captureMessage).toHaveBeenCalledWith('reKolor error: invalidConfig', {
      tags: { kind: 'invalidConfig' },
    });
    // `invocationCallOrder` numbers every call to any mock: `init` came first.
    expect(fake.init.mock.invocationCallOrder[0]).toBeLessThan(
      fake.track.mock.invocationCallOrder[0]!,
    );
  });
});
