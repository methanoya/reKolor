// The Web Worker that owns the WASM engine (T7), exposed to the main thread with Comlink (W6 b).
//
// Generations and revisions: every image the main thread opens gets a new generation number, and
// every pick change a new revision. Results carry both, so the main thread can ignore anything that
// isn't for its current state. Decoding is async, so a slow image A could finish after a newer B:
// a decoded image is committed only if its generation is still the newest requested.

import * as Comlink from 'comlink';
import init, {
  type ConfigPick,
  type ConfigSection,
  type ImageStats,
  type ParsedConfig,
  type ResolvedPick,
  type Mapping,
  type PaletteMatch,
  type Pick,
  type Rgb,
  type Rgba,
} from 'rekolor-wasm';
import wasmUrl from 'rekolor-wasm/rekolor_wasm_bg.wasm?url';
import pantoneText from '../../../palettes/pantone.json?raw';
import { decodeImage, encodePng, toBitmap } from './codec';
import { Engine, type ImageSize } from './engine';
import { err, ok, type AppOutcome } from './outcome';

let engine: Engine | undefined;
/** The newest generation the main thread asked to open. */
let requested = 0;
/** The generation of the image that is open in the engine. */
let current = 0;
/** The last recolor result, kept for the PNG download. */
let output: { generation: number; revision: number; rgba: Uint8Array<ArrayBuffer> } | undefined;

const started: Promise<AppOutcome<{ paletteSize: number }>> = (async () => {
  try {
    await init({ module_or_path: wasmUrl });
  } catch (e) {
    return err('initFailed', `the engine couldn't start: ${String(e)}`);
  }
  const created = Engine.create(pantoneText);
  if (created.status === 'error') return created;
  engine = created.value;
  return ok({ paletteSize: engine.paletteSize });
})();

async function ready(): Promise<AppOutcome<Engine>> {
  const start = await started;
  if (start.status === 'error') return start;
  return engine ? ok(engine) : err('notReady', 'the engine is not ready');
}

const superseded = <T>(): AppOutcome<T> =>
  err('superseded', 'a newer image or change replaced this request');

export interface Opened extends ImageSize {
  generation: number;
  /** The decoded image, for display (transferred). */
  bitmap: ImageBitmap;
}

export interface Recolored {
  generation: number;
  revision: number;
  /** The recolored image, for display (transferred). */
  bitmap: ImageBitmap;
}

const api = {
  /** Resolves once WASM and the palette are loaded (or failed to load). */
  ready: () => started,

  /** Decodes and opens an image file as `generation` (which must be newer than any before). */
  async open(file: Blob, generation: number): Promise<AppOutcome<Opened>> {
    requested = Math.max(requested, generation);
    const engine = await ready();
    if (engine.status === 'error') return engine;
    const decoded = await decodeImage(file);
    if (decoded.status === 'error') return decoded;
    const { rgba, width, height, bitmap } = decoded.value;
    if (generation !== requested) {
      bitmap.close();
      return superseded();
    }
    const opened = engine.value.open(rgba, width, height);
    if (opened.status === 'error') {
      bitmap.close();
      return opened;
    }
    current = generation;
    output = undefined;
    return ok(Comlink.transfer({ generation, width, height, bitmap }, [bitmap]));
  },

  /** Color counts of the open image (separate from `open`: it can take a moment). */
  async analyze(generation: number): Promise<AppOutcome<ImageStats>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    return generation === current ? engine.value.analyze() : superseded();
  },

  async pick(generation: number, x: number, y: number, seen?: Rgba): Promise<AppOutcome<Pick>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    return generation === current ? engine.value.pick(x, y, seen) : superseded();
  },

  async nearest(color: Rgb, k?: number): Promise<AppOutcome<PaletteMatch[]>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    return engine.value.nearest(color, k);
  },

  async recolor(
    generation: number,
    revision: number,
    mappings: Mapping[],
  ): Promise<AppOutcome<Recolored>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    if (generation !== current) return superseded();
    const size = engine.value.image;
    if (!size) return superseded();
    const result = engine.value.recolor(mappings);
    if (result.status === 'error') return result;
    output = { generation, revision, rgba: result.value };
    const bitmap = await toBitmap(result.value, size.width, size.height);
    return ok(Comlink.transfer({ generation, revision, bitmap }, [bitmap]));
  },

  async parseConfig(text: string): Promise<AppOutcome<ParsedConfig>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    return engine.value.parseConfig(text);
  },

  /**
   * Resolves a config section, with the ink alternatives for each pick (the nearest ones, plus the
   * config's own ink if it isn't among them).
   */
  async resolveSection(
    section: ConfigSection,
  ): Promise<AppOutcome<(ResolvedPick & { alternatives: PaletteMatch[] })[]>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    const resolved = engine.value.resolveSection(section);
    if (resolved.status === 'error') return resolved;
    return ok(
      resolved.value.map((pick) => {
        const nearest = engine.value.nearest(pick.matching);
        const alternatives = nearest.status === 'ok' ? nearest.value : [];
        if (!alternatives.some((a) => a.index === pick.ink.index)) alternatives.push(pick.ink);
        return { ...pick, alternatives };
      }),
    );
  },

  async exportConfig(imageName: string, picks: ConfigPick[]): Promise<AppOutcome<string>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    return engine.value.exportConfig(imageName, picks);
  },

  /** The PNG of the recolor result for exactly this generation and revision. */
  async encodePng(generation: number, revision: number): Promise<AppOutcome<Blob>> {
    const engine = await ready();
    if (engine.status === 'error') return engine;
    const size = engine.value.image;
    if (!output || !size || output.generation !== generation || output.revision !== revision) {
      return superseded();
    }
    return ok(await encodePng(output.rgba, size.width, size.height));
  },
};

export type WorkerApi = typeof api;

Comlink.expose(api);
