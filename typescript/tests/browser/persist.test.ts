// The storage behind "a reload brings back the work" (`src/lib/persist.ts`), in a real browser:
// IndexedDB for the image, `sessionStorage` for the tab's ID, and a Web Lock per open tab. Other
// tabs are simulated on this page: an image stored under another ID, and a lock held for it (an
// open tab) or not (a closed one). `fresh()` loads a new copy of the module, as a newly loaded
// page would have.
import { describe, expect, test } from 'vitest';

type Persist = typeof import('../../src/lib/persist');
let copies = 0;
/**
 * A new copy of the module, as a newly loaded page has: the browser caches modules by URL, so a
 * new query string makes it load the file again (`vi.resetModules` can't reset the browser's
 * cache).
 */
const fresh = (): Promise<Persist> =>
  import(/* @vite-ignore */ `../../src/lib/persist.ts?copy=${++copies}`) as Promise<Persist>;

// Raw IndexedDB access to the app's store, as another tab's page would have left it.
function store<T>(mode: IDBTransactionMode, f: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open('rekolor', 1);
    open.onupgradeneeded = () => open.result.createObjectStore('images');
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const db = open.result;
      const transaction = db.transaction('images', mode);
      const request = f(transaction.objectStore('images'));
      transaction.oncomplete = () => {
        db.close();
        resolve(request.result);
      };
      transaction.onerror = () => reject(transaction.error);
    };
  });
}
const putRaw = (id: string, name: string) =>
  store('readwrite', (s) =>
    s.put({ bytes: new Uint8Array([1, 2, 3]).buffer, name, type: '', lastModified: 0 }, id),
  );
const keys = async () => (await store('readonly', (s) => s.getAllKeys())).map(String).sort();

/** Holds the lock of tab `id` until `release()` (an open tab). */
function openTab(id: string) {
  let release!: () => void;
  const held = new Promise<void>((resolve) => (release = resolve));
  return new Promise<() => void>((resolve) => {
    void navigator.locks.request(`rekolor.tab.${id}`, () => {
      resolve(release);
      return held;
    });
  });
}

describe('keeping the image per tab', () => {
  test("a closed tab's image is deleted at the next start; open tabs' are kept", async () => {
    const persist = await fresh();
    await persist.saveImage(new File(['mine'], 'mine.png'));
    const mine = await persist.tabId();
    await putRaw('closed-tab', 'old.png');
    const release = await openTab('open-tab');
    await putRaw('open-tab', 'theirs.png');

    await persist.forgetClosedTabs();
    expect(await keys()).toEqual([mine, 'open-tab'].sort());
    expect((await persist.loadImage())?.name).toBe('mine.png');
    release();
  });

  test('the newest image save wins, even when an older one finishes reading later', async () => {
    const persist = await fresh();
    // The older file is slow to read (a large image): it finishes after the newer one is saved.
    let release!: () => void;
    const slow = new Promise<void>((resolve) => (release = resolve));
    const older = new File(['older'], 'older.png');
    Object.defineProperty(older, 'arrayBuffer', {
      value: async () => {
        await slow;
        return Blob.prototype.arrayBuffer.call(older);
      },
    });
    const olderSaved = persist.saveImage(older);
    expect(await persist.saveImage(new File(['newer'], 'newer.png'))).toBe(true);
    release();
    expect(await olderSaved).toBe(false);
    expect((await persist.loadImage())?.name).toBe('newer.png');
  });

  test('without Web Locks the image is not kept: nothing could delete it later', async () => {
    // A browser without Web Locks (or a page not served over HTTPS).
    Object.defineProperty(navigator, 'locks', { value: undefined, configurable: true });
    try {
      const persist = await fresh();
      expect(await persist.saveImage(new File(['x'], 'x.png'))).toBe(false);
      expect(await persist.loadImage()).toBeUndefined();
      expect(await keys()).toEqual([]);
    } finally {
      // Removes the stand-in, so `navigator.locks` is the browser's again.
      delete (navigator as unknown as Record<string, unknown>).locks;
    }
  });

  test('a duplicated tab takes a new ID and a copy of the image', async () => {
    // The original tab: its ID in sessionStorage (copied into the duplicate) and its lock held.
    sessionStorage.setItem('rekolor.tab', 'original');
    const release = await openTab('original');
    await putRaw('original', 'shared.png');

    const persist = await fresh();
    const id = await persist.tabId();
    expect(id).not.toBe('original');
    expect(sessionStorage.getItem('rekolor.tab')).toBe(id);
    expect((await persist.loadImage())?.name).toBe('shared.png');
    // The original keeps its own.
    expect(await keys()).toContain('original');
    release();
  });
});
