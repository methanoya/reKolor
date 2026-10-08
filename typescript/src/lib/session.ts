// The worker's state and rules, without the worker: generations, revisions, the recolor output
// kept for download, and every call's guards. The image codec is injected, so Node tests can
// drive the races deterministically (a slow decode, an open during a PNG encode); the worker
// (`worker.ts`) passes the browser codec and adds Comlink transfers.
//
// Generations and revisions: every image the main thread opens gets a new generation number, and
// every pick change a new revision. Results carry both, so the main thread can ignore anything
// that isn't for its current state. Decoding and encoding are async, so:
// - a decoded image is committed only if its generation is still the newest requested (a slow
//   image A can't replace a newer B);
// - a PNG is returned only if the image and recolor result are still the ones it was made from.

import type {
  ConfigPick,
  ConfigSection,
  Mapping,
  MaterialRange,
  PaletteMatch,
  ParsedConfig,
  Pick,
  ResolvedPick,
  Rgb,
  Rgba,
} from 'rekolor-wasm';
import type { Decoded } from './codec';
import type { Engine, ImageSize } from './engine';
import { err, ok, type AppOutcome } from './outcome';
import { sameRgb } from './picks';

export interface Codec {
  decode(file: Blob): Promise<AppOutcome<Decoded>>;
  toBitmap(rgba: Uint8Array<ArrayBuffer>, width: number, height: number): Promise<ImageBitmap>;
  encodePng(rgba: Uint8Array<ArrayBuffer>, width: number, height: number): Promise<Blob>;
}

export interface Opened extends ImageSize {
  generation: number;
  /** The decoded image, for display. */
  bitmap: ImageBitmap;
}

export interface Recolored {
  generation: number;
  revision: number;
  /** The recolored image, for display. */
  bitmap: ImageBitmap;
}

/** A pick re-matched on a new material (`Session.rematch`). */
export interface Rematched {
  /** The pixel composited over the new material. */
  matching: Rgb;
  /**
   * Only when `matching` changed: the nearest inks, nearest first (the first is the suggestion, as
   * for a click). Absent: the pick keeps its ink and alternatives (M3 a).
   */
  alternatives?: PaletteMatch[];
}

const superseded = <T>(): AppOutcome<T> =>
  err('superseded', 'a newer image or change replaced this request');

export class Session {
  readonly #engine: Engine;
  readonly #codec: Codec;
  /** The newest generation the main thread asked to open. */
  #requested = 0;
  /** The generation of the image that is open in the engine. */
  #current = 0;
  /** The last recolor result, kept for the PNG download. */
  #output: { generation: number; revision: number; rgba: Uint8Array<ArrayBuffer> } | undefined;

  constructor(engine: Engine, codec: Codec) {
    this.#engine = engine;
    this.#codec = codec;
  }

  /** Decodes and opens an image file as `generation` (which must be newer than any before). */
  async open(file: Blob, generation: number): Promise<AppOutcome<Opened>> {
    this.#requested = Math.max(this.#requested, generation);
    const decoded = await this.#codec.decode(file);
    if (decoded.status === 'error') return decoded;
    const { rgba, width, height, bitmap } = decoded.value;
    if (generation !== this.#requested) {
      bitmap.close();
      return superseded();
    }
    const opened = this.#engine.open(rgba, width, height);
    if (opened.status === 'error') {
      bitmap.close();
      return opened;
    }
    this.#current = generation;
    this.#output = undefined;
    return ok({ generation, width, height, bitmap });
  }

  colorCount(generation: number, material: Rgb): AppOutcome<number> {
    return generation === this.#current ? this.#engine.colorCount(material) : superseded();
  }

  pick(generation: number, x: number, y: number, material: Rgb, seen?: Rgba): AppOutcome<Pick> {
    return generation === this.#current ? this.#engine.pick(x, y, material, seen) : superseded();
  }

  /**
   * Re-matches picks on a new material (M3 a), in order: each pick's pixel composited over it and,
   * where that color differs from the pick's current `matching`, the nearest inks.
   */
  rematch(picks: { pixel: Rgba; matching: Rgb }[], material: Rgb): AppOutcome<Rematched[]> {
    const rematched: Rematched[] = [];
    for (const pick of picks) {
      const matching = this.#engine.composite(pick.pixel, material);
      if (matching.status === 'error') return matching;
      if (sameRgb(matching.value, pick.matching)) {
        rematched.push({ matching: matching.value });
        continue;
      }
      const alternatives = this.#engine.nearest(matching.value);
      if (alternatives.status === 'error') return alternatives;
      rematched.push({ matching: matching.value, alternatives: alternatives.value });
    }
    return ok(rematched);
  }

  nearest(color: Rgb, k?: number): AppOutcome<PaletteMatch[]> {
    return this.#engine.nearest(color, k);
  }

  async recolor(
    generation: number,
    revision: number,
    mappings: Mapping[],
    material: Rgb,
    materialRanges: MaterialRange[] = [],
  ): Promise<AppOutcome<Recolored>> {
    const size = this.#engine.image;
    if (generation !== this.#current || !size) return superseded();
    const result = this.#engine.recolor(mappings, material, materialRanges);
    if (result.status === 'error') return result;
    this.#output = { generation, revision, rgba: result.value };
    const bitmap = await this.#codec.toBitmap(result.value, size.width, size.height);
    return ok({ generation, revision, bitmap });
  }

  /** The PNG of the recolor result for exactly this generation and revision. */
  async encodePng(generation: number, revision: number): Promise<AppOutcome<Blob>> {
    const size = this.#engine.image;
    const output = this.#output;
    if (!output || !size || output.generation !== generation || output.revision !== revision) {
      return superseded();
    }
    const png = await this.#codec.encodePng(output.rgba, size.width, size.height);
    // Encoding is async: a newer image or recolor may have arrived meanwhile (review fix).
    return this.#output === output && this.#current === generation ? ok(png) : superseded();
  }

  parseConfig(text: string): AppOutcome<ParsedConfig> {
    return this.#engine.parseConfig(text);
  }

  /**
   * Resolves a config section on the material (the config's), with the ink alternatives for each
   * pick (the nearest ones, plus the config's own ink if it isn't among them).
   */
  resolveSection(
    section: ConfigSection,
    material: Rgb,
  ): AppOutcome<(ResolvedPick & { alternatives: PaletteMatch[] })[]> {
    const resolved = this.#engine.resolveSection(section, material);
    if (resolved.status === 'error') return resolved;
    return ok(
      resolved.value.map((pick) => {
        const nearest = this.#engine.nearest(pick.matching);
        const alternatives = nearest.status === 'ok' ? nearest.value : [];
        if (!alternatives.some((a) => a.index === pick.ink.index)) alternatives.push(pick.ink);
        return { ...pick, alternatives };
      }),
    );
  }

  exportConfig(imageName: string, material: Rgb, picks: ConfigPick[]): AppOutcome<string> {
    return this.#engine.exportConfig(imageName, material, picks);
  }
}
