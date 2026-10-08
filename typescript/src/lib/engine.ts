// The engine: owns the WASM objects (palette, current image) and wraps every call in an
// `AppOutcome`. Plain TypeScript with no browser APIs, so it runs in Node tests against the real
// WASM package; the worker adds decoding and Comlink around it. The WASM module must be
// initialized before an engine is created.

import {
  Palette,
  SourceImage,
  parseConfig,
  serializeConfig,
  type ConfigPick,
  type ConfigSection,
  type ImageStats,
  type ParsedConfig,
  type ResolvedPick,
  type Mapping,
  type Outcome,
  type PaletteMatch,
  type Pick,
  type Rgb,
  type Rgba,
} from 'rekolor-wasm';
import { imageLimitError } from './limits';
import { err, ok, type AppOutcome } from './outcome';
import { parsePaletteJson } from './palette';

/** Number of ink alternatives offered per pick (owner decision WA2 a). */
export const ALTERNATIVES = 8;

export interface ImageSize {
  width: number;
  height: number;
}

const fromWasm = <T>(outcome: Outcome<T>): AppOutcome<T> =>
  outcome.status === 'ok' ? ok(outcome.value) : { status: 'error', error: outcome.error };

export class Engine {
  #palette: Palette;
  #image: SourceImage | undefined;

  private constructor(palette: Palette) {
    this.#palette = palette;
  }

  /** Creates the engine from the palette JSON (`palettes/pantone.json` format, file order kept). */
  static create(paletteJson: string): AppOutcome<Engine> {
    let data;
    try {
      data = parsePaletteJson(paletteJson);
    } catch (e) {
      return err('initFailed', `the palette couldn't be read: ${(e as Error).message}`);
    }
    const palette = Palette.create(data);
    return palette.status === 'ok'
      ? ok(new Engine(palette.value))
      : err('initFailed', palette.error.message);
  }

  get paletteSize(): number {
    return this.#palette.length;
  }

  get image(): ImageSize | undefined {
    return this.#image && { width: this.#image.width, height: this.#image.height };
  }

  /**
   * Replaces the current image with straight-alpha RGBA pixels (copied into WASM). On error the
   * previous image stays open.
   */
  open(rgba: Uint8Array, width: number, height: number): AppOutcome<ImageSize> {
    const tooLarge = imageLimitError(width, height);
    if (tooLarge) return err('imageTooLarge', tooLarge);
    const created = SourceImage.create(rgba, width, height);
    if (created.status === 'error') return { status: 'error', error: created.error };
    this.#image?.free();
    this.#image = created.value;
    return ok({ width, height });
  }

  /** Closes the current image and frees its pixels. */
  close(): void {
    this.#image?.free();
    this.#image = undefined;
  }

  /** Frees everything; the engine can't be used afterwards. */
  dispose(): void {
    this.close();
    this.#palette.free();
  }

  /** Size and color counts of the current image. Memory grows with distinct RGBA values. */
  analyze(): AppOutcome<ImageStats> {
    return this.#image ? ok(this.#image.analyze()) : err('noImage', 'no image is open');
  }

  /** Distinct colors after compositing over white, with fixed memory (what the app shows). */
  colorCount(): AppOutcome<number> {
    return this.#image ? ok(this.#image.colorCount()) : err('noImage', 'no image is open');
  }

  /** The stored pixel at (x, y), its matching color and suggested ink (R10). */
  pick(x: number, y: number, seen?: Rgba): AppOutcome<Pick> {
    if (!this.#image) return err('noImage', 'no image is open');
    return fromWasm(this.#image.pick(x, y, seen, this.#palette));
  }

  /** The palette entries nearest to a color, nearest first (ties: file order). */
  nearest(color: Rgb, k: number = ALTERNATIVES): AppOutcome<PaletteMatch[]> {
    const outcome = fromWasm(this.#palette.nearest(color, k));
    return outcome.status === 'ok' ? ok(outcome.value.matches) : outcome;
  }

  /** Parses and validates a palette config (`*.palettes.toml`, W10 v), like the CLI does. */
  parseConfig(text: string): AppOutcome<ParsedConfig> {
    return fromWasm(parseConfig(text));
  }

  /** Resolves a config section against the palette (unknown inks are errors), in pick order. */
  resolveSection(section: ConfigSection): AppOutcome<ResolvedPick[]> {
    const outcome = fromWasm(this.#palette.resolveSection(section));
    return outcome.status === 'ok' ? ok(outcome.value.picks) : outcome;
  }

  /** The picks as a config file with one section (`size` = the number of picks). */
  exportConfig(imageName: string, picks: ConfigPick[]): AppOutcome<string> {
    const outcome = fromWasm(serializeConfig({ imageName, picks }));
    return outcome.status === 'ok' ? ok(outcome.value.text) : outcome;
  }

  /** Recolors the current image into a new opaque RGBA buffer (transferable to the main thread). */
  recolor(mappings: Mapping[]): AppOutcome<Uint8Array<ArrayBuffer>> {
    if (!this.#image) return err('noImage', 'no image is open');
    const out = new Uint8Array(this.#image.width * this.#image.height * 4);
    const outcome = fromWasm(this.#image.recolor({ mappings }, out));
    return outcome.status === 'ok' ? ok(out) : outcome;
  }
}
