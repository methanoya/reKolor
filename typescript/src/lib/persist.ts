// Keeps this tab's work across reloads, in the browser only (nothing is sent anywhere):
// - the picks, the material, the unprinted colors and an uploaded palette in `sessionStorage`
//   (one per tab, cleared when the tab closes, about 5 MB);
// - the open image in IndexedDB (the browser's database, for files too large for
//   `sessionStorage`), under an ID that belongs to this tab.
//
// Tying IndexedDB to the tab: `sessionStorage` holds the tab's ID, and while the page is open it
// holds a Web Lock (`navigator.locks`) named after that ID. When a page starts, an image whose ID
// no open page holds belongs to a closed tab and is deleted (`forgetClosedTabs`). A duplicated
// tab gets a copy of `sessionStorage`, ID included; it finds that lock taken, so it takes a new ID
// and a copy of the image.
//
// Storage can be unavailable (a private window, blocked site data) or full: every function here
// then does nothing, and the app works as before, just without restoring.
import type { PaletteMatch, Rgb, Rgba } from 'rekolor-wasm';
import { LIMITS } from './limits';
import type { PickEntry, RangeEntry } from './picks';

const STATE_KEY = 'rekolor.session';
const TAB_KEY = 'rekolor.tab';
const LOCK_PREFIX = 'rekolor.tab.';
const DB_NAME = 'rekolor';
const STORE = 'images';
/**
 * How long a starting page waits before deleting an image whose tab holds no lock: a tab that is
 * reloading takes its lock back well within that.
 */
const ORPHAN_GRACE_MS = 3_000;

/** What `sessionStorage` keeps for this tab. */
export interface SavedState {
  version: 1;
  /** The open image's name and size: the image in IndexedDB is restored only if it matches. */
  image?: { name: string; size: number };
  /** An uploaded palette (absent: Pantone). */
  palette?: { name: string; text: string };
  material: Rgb;
  materialChosen: boolean;
  picks: PickEntry[];
  ranges: RangeEntry[];
}

// ── sessionStorage ────────────────────────────────────────────────────────────────────────────────

/**
 * Saves the state. If it doesn't fit (a large palette) or storage is unavailable, the previous save
 * stays as it was: a reload then brings back the last state that fitted.
 */
export function saveState(state: SavedState): void {
  try {
    sessionStorage.setItem(STATE_KEY, JSON.stringify(state));
  } catch {
    // Too large, or storage unavailable: keep the previous save.
  }
}

/** The saved state, if there is one and it is well formed. */
export function loadState(): SavedState | undefined {
  try {
    const text = sessionStorage.getItem(STATE_KEY);
    return text === null ? undefined : parseSavedState(text);
  } catch {
    return undefined;
  }
}

// The saved text comes from an earlier page of this app, but it may be from an older version, or
// edited by hand in the browser's tools: check its shape before using it (`v is T` tells
// TypeScript the value has that type when the function returns true).
const isObject = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);
const isByte = (v: unknown) => Number.isInteger(v) && (v as number) >= 0 && (v as number) <= 255;
const isNumber = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const isRgb = (v: unknown): v is Rgb => isObject(v) && isByte(v.r) && isByte(v.g) && isByte(v.b);
const isRgba = (v: unknown): v is Rgba => isRgb(v) && isByte((v as Rgba).a);
const isDeltaE = (v: unknown) => isNumber(v) && v >= 0 && v <= 100;
const isPoint = (v: unknown) => v === undefined || (isObject(v) && isNumber(v.x) && isNumber(v.y));
const isMatch = (v: unknown): v is PaletteMatch =>
  isObject(v) &&
  Number.isInteger(v.index) &&
  typeof v.name === 'string' &&
  isRgb(v.rgb) &&
  typeof v.nonPalette === 'boolean' &&
  isNumber(v.deltaE);
const isPick = (v: unknown): v is PickEntry =>
  isObject(v) &&
  Number.isInteger(v.id) &&
  isRgba(v.pixel) &&
  isRgb(v.matching) &&
  isMatch(v.ink) &&
  Array.isArray(v.alternatives) &&
  v.alternatives.every(isMatch) &&
  isDeltaE(v.deltaE) &&
  isNumber(v.maxDeltaE) &&
  isPoint(v.at) &&
  (v.mismatch === undefined ||
    (isObject(v.mismatch) &&
      isRgba(v.mismatch.seen) &&
      isRgba(v.mismatch.stored) &&
      isNumber(v.mismatch.maxChannelDifference)));
const isRange = (v: unknown): v is RangeEntry =>
  isObject(v) &&
  Number.isInteger(v.id) &&
  isRgba(v.pixel) &&
  isDeltaE(v.deltaE) &&
  isNumber(v.maxDeltaE) &&
  isPoint(v.at) &&
  (v.material === undefined || typeof v.material === 'boolean');

/** Reads saved state text; `undefined` if anything in it is not as this version writes it. */
export function parseSavedState(text: string): SavedState | undefined {
  let v: unknown;
  try {
    v = JSON.parse(text);
  } catch {
    return undefined;
  }
  if (
    !isObject(v) ||
    v.version !== 1 ||
    !isRgb(v.material) ||
    typeof v.materialChosen !== 'boolean' ||
    !Array.isArray(v.picks) ||
    v.picks.length > LIMITS.picks ||
    !v.picks.every(isPick) ||
    !Array.isArray(v.ranges) ||
    v.ranges.length > LIMITS.unprinted ||
    !v.ranges.every(isRange) ||
    !(
      v.image === undefined ||
      (isObject(v.image) && typeof v.image.name === 'string' && isNumber(v.image.size))
    ) ||
    !(
      v.palette === undefined ||
      (isObject(v.palette) &&
        typeof v.palette.name === 'string' &&
        typeof v.palette.text === 'string')
    )
  ) {
    return undefined;
  }
  return v as unknown as SavedState;
}

// ── The tab's ID and lock ─────────────────────────────────────────────────────────────────────────

// Once per page: the ID is claimed (and its lock taken) by the first call, and reused after that.
let tab: Promise<string | undefined> | undefined;

/** This tab's ID, holding its lock; `undefined` without storage. */
export function tabId(): Promise<string | undefined> {
  tab ??= claimTab();
  return tab;
}

async function claimTab(): Promise<string | undefined> {
  try {
    const saved = sessionStorage.getItem(TAB_KEY);
    // Without Web Locks (an old browser), the ID still works; closed tabs just aren't cleaned up.
    if (!('locks' in navigator)) {
      const id = saved ?? crypto.randomUUID();
      sessionStorage.setItem(TAB_KEY, id);
      return id;
    }
    if (saved && (await hold(saved))) return saved;
    // No ID yet, or a duplicated tab (its ID is held by the original): a new ID, with a copy of
    // the original tab's image.
    const id = crypto.randomUUID();
    if (saved) {
      const image = await getImage(saved).catch(() => undefined);
      if (image) await putImage(id, image).catch(() => undefined);
    }
    sessionStorage.setItem(TAB_KEY, id);
    await hold(id);
    return id;
  } catch {
    return undefined;
  }
}

/**
 * Takes the lock named after `id` for as long as the page is open, if no other page holds it.
 * Resolves to whether it was taken. (The lock is released when the page closes or reloads.)
 */
function hold(id: string): Promise<boolean> {
  return new Promise((resolve) => {
    void navigator.locks.request(LOCK_PREFIX + id, { ifAvailable: true }, (lock) => {
      resolve(lock !== null);
      // Returning a promise that never settles keeps the lock until the page goes away.
      return lock ? new Promise<never>(() => {}) : undefined;
    });
  });
}

// ── IndexedDB: the image ──────────────────────────────────────────────────────────────────────────

// Saves are numbered, so the newest one wins: reading a large file takes a while, and an older
// save that finishes reading after a newer one must not overwrite it.
let latestSave = 0;

/**
 * Saves the open image for this tab (replacing the previous one). Resolves to whether this file is
 * now the stored image: `false` if storage failed or a newer save started meanwhile.
 */
export async function saveImage(file: File): Promise<boolean> {
  const save = ++latestSave;
  try {
    const id = await tabId();
    if (!id) return false;
    const stored = await toStored(file);
    if (save !== latestSave) return false;
    // Writes run in the order they start, so a newer save's write, started later, lands last.
    await putStored(id, stored);
    return save === latestSave;
  } catch {
    return false;
  }
}

/** This tab's saved image, if any. */
export async function loadImage(): Promise<File | undefined> {
  const id = await tabId();
  return id ? getImage(id).catch(() => undefined) : undefined;
}

/**
 * Deletes the images of closed tabs: those whose ID no open page holds, now and again after
 * `ORPHAN_GRACE_MS` (a reloading tab takes its lock back in between).
 */
export async function forgetClosedTabs(): Promise<void> {
  if (!('locks' in navigator)) return;
  try {
    const id = await tabId();
    const orphans = async () => {
      const held = new Set(((await navigator.locks.query()).held ?? []).map((l) => l.name));
      return (await allKeys()).filter((key) => key !== id && !held.has(LOCK_PREFIX + key));
    };
    if ((await orphans()).length === 0) return;
    await new Promise((resolve) => setTimeout(resolve, ORPHAN_GRACE_MS));
    for (const key of await orphans()) await deleteImage(key);
  } catch {
    // Storage unavailable: nothing to clean.
  }
}

// IndexedDB's API reports results through events; these helpers turn one request into a promise.
// Each opens the database, runs one request in a transaction, and closes it again.
function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, 1);
    // First use (or a newer version): create the store; keys are tab IDs, values the images.
    request.onupgradeneeded = () => request.result.createObjectStore(STORE);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('IndexedDB unavailable'));
  });
}

async function run<T>(
  mode: IDBTransactionMode,
  f: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await openDb();
  try {
    return await new Promise<T>((resolve, reject) => {
      const transaction = db.transaction(STORE, mode);
      const request = f(transaction.objectStore(STORE));
      transaction.oncomplete = () => resolve(request.result);
      transaction.onerror = transaction.onabort = () =>
        reject(transaction.error ?? new Error('IndexedDB transaction failed'));
    });
  } finally {
    db.close();
  }
}

// The image is stored as its bytes plus its name, type and date, not as the `File` itself: WebKit
// can't store a `File` (or `Blob`) in IndexedDB in a private window, but bytes it can.
interface StoredImage {
  bytes: ArrayBuffer;
  name: string;
  type: string;
  lastModified: number;
}

const toStored = async (file: File): Promise<StoredImage> => ({
  bytes: await file.arrayBuffer(),
  name: file.name,
  type: file.type,
  lastModified: file.lastModified,
});
const putStored = async (id: string, stored: StoredImage) => {
  await run('readwrite', (s) => s.put(stored, id));
};
const putImage = async (id: string, file: File) => putStored(id, await toStored(file));
const getImage = async (id: string) => {
  const value: unknown = await run('readonly', (s) => s.get(id));
  if (!isObject(value) || !(value.bytes instanceof ArrayBuffer)) return undefined;
  const { bytes, name, type, lastModified } = value as unknown as StoredImage;
  return new File([bytes], name, { type, lastModified });
};
const deleteImage = (id: string) => run('readwrite', (s) => s.delete(id));
const allKeys = async () => (await run('readonly', (s) => s.getAllKeys())).map(String);
