// The engine: owns the WASM objects (palette, current image) and wraps every call in an
// `AppOutcome`. Plain TypeScript with no browser APIs, so it runs in Node tests against the real
// WASM package; the worker adds decoding and Comlink around it. The WASM module must be
// initialized before an engine is created.

import {
  Palette,
  SourceImage,
  composite,
  parseConfig,
  serializeConfig,
  unprintedColors,
  type ConfigPick,
  type ConfigSection,
  type ConfigUnprinted,
  type ImageStats,
  type ParsedConfig,
  type ResolvedPick,
  type Mapping,
  type MaterialRange,
  type Outcome,
  type PaletteMatch,
  type Pick,
  type Rgb,
  type Rgba,
} from 'rekolor-wasm';
import { imageLimitError } from './limits';
import { err, ok, type AppOutcome } from './outcome';
import { parsePaletteJson } from './palette';

/** Number of ink alternatives offered per pick. */
export const ALTERNATIVES = 8;

export interface ImageSize {
  width: number;
  height: number;
}

// The WASM package's outcome as the app's outcome type (the same shape; the app's error kinds are a
// superset).
const fromWasm = <T>(outcome: Outcome<T>): AppOutcome<T> =>
  outcome.status === 'ok' ? ok(outcome.value) : { status: 'error', error: outcome.error };

// The WASM objects live in WASM memory, which JavaScript's garbage collector doesn't see; each is
// released with `free()` when it is replaced or the engine is disposed.
export class Engine {
  #palette: Palette;
  #image: SourceImage | undefined;

  // `private constructor`: the only way to get an engine is `Engine.create`, which can report an
  // error (a constructor can't return one).
  private constructor(palette: Palette) {
    this.#palette = palette;
  }

  /** Creates the engine from the palette JSON (`palettes/pantone.json` format, file order kept). */
  static create(paletteJson: string): AppOutcome<Engine> {
    // `data`'s type is inferred from the assignment in `try`.
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

  /**
   * Replaces the palette with another one (`palettes/pantone.json` format), keeping the open
   * image. On error (an `invalidPalette` outcome) the current palette stays.
   */
  setPalette(paletteJson: string): AppOutcome<number> {
    let data;
    try {
      data = parsePaletteJson(paletteJson);
    } catch (e) {
      return err('invalidPalette', (e as Error).message);
    }
    const palette = Palette.create(data);
    if (palette.status === 'error') return err('invalidPalette', palette.error.message);
    this.#palette.free();
    this.#palette = palette.value;
    return ok(palette.value.length);
  }

  get image(): ImageSize | undefined {
    // `a && b` gives `a` if it is falsy (here `undefined`), otherwise `b`.
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
    // `?.` calls `free()` only if there is an image.
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

  /**
   * Size and color counts of the current image, `colors` on the material. Memory grows with
   * distinct RGBA values.
   */
  analyze(material: Rgb): AppOutcome<ImageStats> {
    if (!this.#image) return err('noImage', 'no image is open');
    return fromWasm(this.#image.analyze(material));
  }

  /** Distinct colors after compositing over the material, with fixed memory (what the app shows). */
  colorCount(material: Rgb): AppOutcome<number> {
    if (!this.#image) return err('noImage', 'no image is open');
    const outcome = fromWasm(this.#image.colorCount(material));
    return outcome.status === 'ok' ? ok(outcome.value.colors) : outcome;
  }

  /** The stored pixel at (x, y), its matching color on the material and suggested ink. */
  pick(x: number, y: number, material: Rgb, seen?: Rgba): AppOutcome<Pick> {
    if (!this.#image) return err('noImage', 'no image is open');
    return fromWasm(this.#image.pick(x, y, seen, material, this.#palette));
  }

  /** A pixel composited over the material: the color recolor matches (computed in Rust). */
  composite(pixel: Rgba, material: Rgb): AppOutcome<Rgb> {
    return fromWasm(composite(pixel, material));
  }

  /**
   * Which colors (as recolor matches them, e.g. the picks' `matching`) the material ranges leave
   * unprinted on the material: one flag per color, by recolor's own test (computed in Rust).
   */
  unprintedColors(
    colors: Rgb[],
    material: Rgb,
    materialRanges: MaterialRange[],
  ): AppOutcome<boolean[]> {
    const outcome = fromWasm(unprintedColors({ colors, material, materialRanges }));
    return outcome.status === 'ok' ? ok(outcome.value.unprinted) : outcome;
  }

  // `k: number = ALTERNATIVES` is a default parameter value, used when `k` is omitted.
  /** The palette entries nearest to a color, nearest first (ties: file order). */
  nearest(color: Rgb, k: number = ALTERNATIVES): AppOutcome<PaletteMatch[]> {
    const outcome = fromWasm(this.#palette.nearest(color, k));
    return outcome.status === 'ok' ? ok(outcome.value.matches) : outcome;
  }

  /** Parses and validates a palette config (`*.palettes.toml`), like the CLI does. */
  parseConfig(text: string): AppOutcome<ParsedConfig> {
    return fromWasm(parseConfig(text));
  }

  /**
   * Resolves a config section against the palette (unknown inks are errors), in pick order, with
   * each pick composited over the material (the config's).
   */
  resolveSection(section: ConfigSection, material: Rgb): AppOutcome<ResolvedPick[]> {
    const outcome = fromWasm(this.#palette.resolveSection(section, material));
    return outcome.status === 'ok' ? ok(outcome.value.picks) : outcome;
  }

  /**
   * The picks as a config file with the material, the unprinted colors and one section (`size` =
   * the number of picks).
   */
  exportConfig(
    imageName: string,
    material: Rgb,
    picks: ConfigPick[],
    unprinted: ConfigUnprinted[] = [],
  ): AppOutcome<string> {
    const outcome = fromWasm(serializeConfig({ imageName, material, unprinted, picks }));
    return outcome.status === 'ok' ? ok(outcome.value.text) : outcome;
  }

  /**
   * Recolors the current image on the material into a new RGBA buffer (transferable to the main
   * thread). Pixels are opaque, except those within a material range: they take no ink and are
   * transparent.
   */
  recolor(
    mappings: Mapping[],
    material: Rgb,
    materialRanges: MaterialRange[] = [],
  ): AppOutcome<Uint8Array<ArrayBuffer>> {
    if (!this.#image) return err('noImage', 'no image is open');
    // The WASM `recolor` writes straight into this buffer (4 bytes per pixel).
    const out = new Uint8Array(this.#image.width * this.#image.height * 4);
    const outcome = fromWasm(this.#image.recolor({ mappings, material, materialRanges }, out));
    return outcome.status === 'ok' ? ok(out) : outcome;
  }
}
