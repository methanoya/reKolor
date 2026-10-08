// The Web Worker that owns the WASM engine (T7), exposed to the main thread with Comlink (W6 b).
// The rules (generations, revisions, guards) live in `session.ts`; this file starts the engine,
// passes the browser codec, and transfers bitmaps instead of copying them.

import * as Comlink from 'comlink';
import init, {
  type ConfigPick,
  type ConfigSection,
  type ConfigUnprinted,
  type Mapping,
  type MaterialRange,
  type Rgb,
  type Rgba,
} from 'rekolor-wasm';
import wasmUrl from 'rekolor-wasm/rekolor_wasm_bg.wasm?url';
import pantoneText from '../../../palettes/pantone.json?raw';
import { decodeImage, encodePng, toBitmap } from './codec';
import { Engine } from './engine';
import { err, ok, type AppOutcome } from './outcome';
import { Session } from './session';

let session: Session | undefined;

const started: Promise<AppOutcome<{ paletteSize: number }>> = (async () => {
  try {
    await init({ module_or_path: wasmUrl });
  } catch (e) {
    return err('initFailed', `the engine couldn't start: ${String(e)}`);
  }
  const created = Engine.create(pantoneText);
  if (created.status === 'error') return created;
  session = new Session(created.value, { decode: decodeImage, toBitmap, encodePng });
  return ok({ paletteSize: created.value.paletteSize });
})();

/** Runs `f` once the engine is ready (or returns why it isn't). */
async function withSession<T>(
  f: (session: Session) => AppOutcome<T> | Promise<AppOutcome<T>>,
): Promise<AppOutcome<T>> {
  const start = await started;
  if (start.status === 'error') return start;
  return session ? f(session) : err('notReady', 'the engine is not ready');
}

/** Transfers the bitmap inside an ok outcome (instead of copying it). */
function transferBitmap<T extends { bitmap: ImageBitmap }>(outcome: AppOutcome<T>): AppOutcome<T> {
  return outcome.status === 'ok' ? Comlink.transfer(outcome, [outcome.value.bitmap]) : outcome;
}

const api = {
  /** Resolves once WASM and the palette are loaded (or failed to load). */
  ready: () => started,
  open: async (file: Blob, generation: number) =>
    transferBitmap(await withSession((s) => s.open(file, generation))),
  colorCount: (generation: number, material: Rgb) =>
    withSession((s) => s.colorCount(generation, material)),
  pick: (generation: number, x: number, y: number, material: Rgb, seen?: Rgba) =>
    withSession((s) => s.pick(generation, x, y, material, seen)),
  rematch: (picks: { pixel: Rgba; matching: Rgb }[], material: Rgb) =>
    withSession((s) => s.rematch(picks, material)),
  nearest: (color: Rgb, k?: number) => withSession((s) => s.nearest(color, k)),
  recolor: async (
    generation: number,
    revision: number,
    mappings: Mapping[],
    material: Rgb,
    materialRanges?: MaterialRange[],
  ) =>
    transferBitmap(
      await withSession((s) => s.recolor(generation, revision, mappings, material, materialRanges)),
    ),
  encodePng: (generation: number, revision: number) =>
    withSession((s) => s.encodePng(generation, revision)),
  parseConfig: (text: string) => withSession((s) => s.parseConfig(text)),
  resolveSection: (section: ConfigSection, material: Rgb) =>
    withSession((s) => s.resolveSection(section, material)),
  exportConfig: (
    imageName: string,
    material: Rgb,
    picks: ConfigPick[],
    unprinted?: ConfigUnprinted[],
  ) => withSession((s) => s.exportConfig(imageName, material, picks, unprinted)),
};

export type WorkerApi = typeof api;

Comlink.expose(api);
