<script lang="ts">
  // The root component: the whole page. It owns all of the app's state (the open image, the picks,
  // the material, the unprinted colors, the zoom) and talks to the engine in the Web Worker through
  // `EngineClient`. The child components only display what they are given and report the user's
  // actions back through callbacks.
  //
  // Svelte basics used throughout: `$state(...)` declares a variable whose changes update the page;
  // `$state.raw(...)` does the same but only on reassignment (not when something inside the value
  // changes), which suits values that are always replaced whole, like bitmaps. Plain `let`
  // variables are bookkeeping the page never shows.
  //
  // Every engine call is asynchronous (it goes to the worker and back), so the image or the picks
  // may have changed by the time an answer arrives. The pattern used everywhere: copy the current
  // `generation` (and, for recoloring, `revision`) before the call, and drop the answer if it no
  // longer matches.
  import type { ConfigSection, ConfigUnprinted, Rgb, Rgba } from 'rekolor-wasm';
  import { onDestroy } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import BuiltWith from './components/BuiltWith.svelte';
  import ImageView from './components/ImageView.svelte';
  import PickList from './components/PickList.svelte';
  import { EngineClient } from './lib/client';
  import { ALTERNATIVES } from './lib/engine';
  import { Latest } from './lib/latest';
  import { Mover, type Marker } from './lib/moves';
  import { LIMITS } from './lib/limits';
  import { imageFormat, reportError, track } from './lib/logrocket';
  import type { AppErrorKind, AppOutcome } from './lib/outcome';
  import {
    BLACK,
    WHITE,
    css,
    cssAlpha,
    fromHex,
    hex,
    mappings,
    sameRgb,
    type PickEntry,
    type PickMove,
    type RangeEntry,
  } from './lib/picks';
  import { Serial } from './lib/serial';
  import { fit, resize, zoomAt, type Size, type View } from './lib/view';

  // The privacy page's address (`public/privacy.html`): `BASE_URL` is `/` locally and `/reKolor/`
  // on GitHub Pages.
  const privacyUrl = `${import.meta.env.BASE_URL}privacy.html`;

  // ── Engine ───────────────────────────────────────────────────────────────────────────────────
  // Engine startup and the status/error lines shown under the images.
  let ready = $state(false);
  let status = $state('Starting the engine…');
  let error = $state<string | undefined>();
  // Shows an error message and reports its kind to LogRocket (on the published site only; see
  // `lib/logrocket.ts`): only the kind, never the message, which can name the user's files.
  function showError(kind: AppErrorKind, message: string) {
    error = message;
    reportError(kind);
  }

  // The callback runs when the worker crashed and was restarted: the image is gone with it.
  const client = new EngineClient((reason) => {
    resetImage();
    showError(
      'workerFailed',
      `The engine stopped (${reason}) and was restarted. Open the image again.`,
    );
    void start();
  });

  async function start() {
    ready = false;
    const outcome = await client.call((api) => api.ready());
    if (outcome.status === 'ok') {
      ready = true;
      status = `Engine ready · ${outcome.value.paletteSize} inks`;
    } else {
      status = 'The engine could not start.';
      showError(outcome.error.kind, outcome.error.message);
    }
  }
  // `void` starts the async function without waiting for it (and tells the linter that's intended).
  void start();

  // ── Image state ──────────────────────────────────────────────────────────────────────────────
  /** Incremented per open attempt (the worker requires each to be newer than the last). */
  let requested = 0;
  /**
   * The generation of the image that is open, set only when an open succeeds, so a failed
   * replacement keeps the previous image usable. Results for others are ignored.
   */
  let generation = 0;
  /** Incremented per pick change; only the current revision's result is shown or downloaded. */
  let revision = 0;
  let file = $state<{ name: string; size: number } | undefined>();
  let image = $state<Size | undefined>();
  let colors = $state<number | undefined>();
  // The two displayed images: the decoded original and the latest recolor result.
  let original = $state.raw<ImageBitmap | undefined>();
  let result = $state.raw<ImageBitmap | undefined>();
  let resultRevision = $state(-1);
  let opening = $state(false);
  let recoloring = $state(false);
  let picks = $state<PickEntry[]>([]);
  let nextPickId = 1;
  /** Pick-list changes run one at a time, in the order of the user's actions. */
  const mutations = new Serial();
  /** The pick being Shift-dragged: its marker follows the pointer until the move settles. */
  let moving = $state<Marker>();
  /**
   * The material color (the garment or substrate) every engine call composites over. Changed only
   * inside a `mutations` task, together with the picks re-matched on it, so the two always agree.
   * Kept across new images and worker restarts.
   */
  let material = $state.raw<Rgb>(WHITE);
  // The color input's value, as `#rrggbb` text.
  /** What the color input shows: the user's newest choice, maybe not applied yet. */
  let materialInput = $state(hex(WHITE));
  /**
   * Material inputs so far. An action (a material change, an import) writes `materialInput` only if
   * no input came after it, so the input never jumps back from a newer choice.
   */
  let materialInputs = 0;
  /** Colors left unprinted: pixels within ΔE of them take no ink; the material shows. */
  let ranges = $state<RangeEntry[]>([]);
  let nextRangeId = 1;
  /** Ids of the picks whose own color is left unprinted (shown in the pick list). */
  let unprintedPicks = $state.raw<number[]>([]);
  /** The next click on the original adds a material range instead of a pick. */
  let addingRange = $state(false);
  /**
   * Whether a material was chosen (picker, White, Black, or an import with a non-white one).
   * Until then the material is "none": the frames show the checkerboard, and the engine composites
   * over white as before the material color.
   */
  let materialChosen = $state(false);
  /** The ΔE a new material range starts with. */
  const RANGE_DELTA_E = 10;
  /** The ΔE slider's usual maximum; files may hold up to 100. */
  const RANGE_SLIDER_MAX = 40;

  let view = $state<View>({ scale: 1, x: 0, y: 0 });
  let viewport: Size = { width: 0, height: 0 };

  // Bitmaps hold memory outside JavaScript's heap; `close()` frees it right away.
  /** After a worker restart: nothing is open, nothing is running. */
  function resetImage() {
    generation = 0;
    recolorer.cancel();
    opening = false;
    recoloring = false;
    original?.close();
    result?.close();
    original = undefined;
    result = undefined;
    image = undefined;
    colors = undefined;
    file = undefined;
    picks = [];
    ranges = ranges.filter((r) => r.material);
    addingRange = false;
    mover.reset();
    resultRevision = -1;
  }

  async function openFile(f: File) {
    if (!ready) return;
    const gen = ++requested;
    opening = true;
    error = undefined;
    status = `Opening ${f.name}…`;
    const opened = await client.call((api) => api.open(f, gen));
    // The answer arrived; first check that it is still wanted.
    if (gen !== requested) {
      // A newer open was started meanwhile (or the worker restarted); it owns `opening`.
      if (opened.status === 'ok') opened.value.bitmap.close();
      return;
    }
    opening = false;
    if (opened.status === 'error') {
      if (opened.error.kind !== 'superseded') {
        showError(opened.error.kind, `${f.name}: ${opened.error.message}`);
        status = image ? 'Kept the previous image.' : 'Open an image to start.';
      }
      return;
    }
    generation = gen;
    original?.close();
    result?.close();
    original = opened.value.bitmap;
    result = undefined;
    resultRevision = -1;
    image = { width: opened.value.width, height: opened.value.height };
    file = { name: f.name, size: f.size };
    colors = undefined;
    picks = [];
    ranges = ranges.filter((r) => r.material);
    addingRange = false;
    mover.reset();
    // Show the whole new image, then start counting its colors (shown in the toolbar).
    view = fit(image, viewport);
    status = 'Click a color in the original to pick it.';
    requestRecolor();
    const counted = await countColors();
    // Recorded once the colors are counted, with this open's own count (a material change while it
    // counted starts another count, but this one is still this image's), unless another image was
    // opened meanwhile. `colors` is left out only if counting failed. The format is a name from a
    // fixed list (`imageFormat`), never part of the file's name.
    if (gen === generation) {
      track('Image opened', {
        format: imageFormat(f),
        width: opened.value.width,
        height: opened.value.height,
        ...(counted === undefined ? {} : { colors: counted }),
      });
    }
  }

  /**
   * Counts the current image's colors on the current material (shown in the toolbar), and returns
   * the count (`undefined` if counting failed), even when the toolbar no longer shows it.
   */
  async function countColors(): Promise<number | undefined> {
    const gen = generation;
    const on = material;
    colors = undefined;
    const counted = await client.call((api) => api.colorCount(gen, on));
    if (counted.status !== 'ok') return undefined;
    // Shown only for the image and material it was made for.
    if (gen === generation && on === material) colors = counted.value;
    return counted.value;
  }

  // ── Picks ────────────────────────────────────────────────────────────────────────────────────
  // A click on the original: a new pick (or, after the + button, a new unprinted color). Queued in
  // `mutations`, so it runs after any earlier pick-list change has finished.
  function pickAt(x: number, y: number, seen: Rgba | undefined) {
    if (addingRange) {
      addingRange = false;
      addRange(x, y, seen);
      return;
    }
    const gen = generation;
    void mutations.run(async () => {
      if (gen !== generation) return;
      if (picks.length >= LIMITS.picks) {
        status = `At most ${LIMITS.picks} picks.`;
        return;
      }
      const on = material;
      const picked = await client.call((api) => api.pick(gen, x, y, on, seen));
      if (gen !== generation) return;
      if (picked.status === 'error') {
        showError(picked.error.kind, picked.error.message);
        return;
      }
      // Unpacks the engine's answer into variables.
      const { pixel, matching, suggestion, mismatch } = picked.value;
      if (picks.some((p) => sameRgb(p.matching, matching))) {
        status = `That color (${pixel.r}, ${pixel.g}, ${pixel.b}) is already picked.`;
        return;
      }
      const nearest = await client.call((api) => api.nearest(matching, ALTERNATIVES));
      if (gen !== generation) return;
      const alternatives = nearest.status === 'ok' ? nearest.value : [suggestion];
      // A new array replaces the old one, and Svelte updates everything that shows `picks`.
      // `...picks` copies the existing picks into it; `...(mismatch ? { mismatch } : {})` adds the
      // `mismatch` field only when there is one (an optional field may not be set to `undefined`).
      picks = [
        ...picks,
        {
          id: nextPickId++,
          pixel,
          matching,
          ink: suggestion,
          alternatives,
          at: { x, y },
          ...(mismatch ? { mismatch } : {}),
        },
      ];
      status = `Picked (${pixel.r}, ${pixel.g}, ${pixel.b}) → ${suggestion.name}`;
      requestRecolor();
      track('Pick added', { picks: picks.length });
    });
  }

  // ── Moving a pick (Shift-drag on its marker) ───────────────────────────────────────────────────
  // Live: each new pixel re-picks the color and re-suggests the ink, like a click there, and the
  // preview follows; within the same color the ink is kept. A pixel whose color another pick
  // already has is skipped; on release the pick stays at its last valid pixel. Escape or a
  // lost pointer cancels: the pick is put back as it was. Ordering and coalescing: `Mover`.

  // The `Mover` (see `lib/moves.ts`) decides the order of a drag's updates; these hooks do the
  // actual pick-list work. `$state.snapshot` makes a plain, unproxied copy of reactive state.
  const mover = new Mover<{ pick: PickEntry; material: Rgb }>({
    queue: (task) => mutations.run(task),
    apply: applyMove,
    save: (id) => {
      const pick = picks.find((p) => p.id === id);
      return pick && { pick: $state.snapshot(pick), material };
    },
    restore: async (saved) => {
      let pick = saved?.pick;
      // Saved on the material at the drag's start: re-matched only if it changed since (the picker
      // and a Shift-drag can hardly be used at once, so this is rare).
      if (pick && saved && !sameRgb(saved.material, material)) {
        const gen = generation;
        // `?.[0]`: the first element, or `undefined` if `rematch` failed.
        pick = (await rematch([pick], material))?.[0];
        if (gen !== generation) pick = undefined;
      }
      status = 'Move cancelled.';
      const restored = pick;
      if (!restored || !picks.some((p) => p.id === restored.id)) return;
      picks = picks.map((p) => (p.id === restored.id ? restored : p));
      requestRecolor();
    },
    show: (marker) => (moving = marker),
  });

  /** Re-picks a moved pick at its new pixel (runs inside `mutations`). */
  async function applyMove(move: PickMove) {
    const gen = generation;
    const before = picks.find((p) => p.id === move.id);
    if (!before) return;
    const on = material;
    const picked = await client.call((api) => api.pick(gen, move.x, move.y, on, move.seen));
    if (gen !== generation) return;
    if (picked.status === 'error') {
      if (picked.error.kind !== 'superseded') showError(picked.error.kind, picked.error.message);
      return;
    }
    const { pixel, matching, suggestion, mismatch } = picked.value;
    if (picks.some((p) => p.id !== move.id && sameRgb(p.matching, matching))) {
      if (move.done) {
        status = `That color (${pixel.r}, ${pixel.g}, ${pixel.b}) is already picked; the pick stayed where it was.`;
      }
      return;
    }
    // The same color (a flat area): keep the ink, which may have been chosen by hand.
    const recolored = !sameRgb(before.matching, matching);
    let { ink, alternatives } = before;
    if (recolored) {
      const nearest = await client.call((api) => api.nearest(matching, ALTERNATIVES));
      if (gen !== generation) return;
      ink = suggestion;
      alternatives = nearest.status === 'ok' ? nearest.value : [suggestion];
    }
    picks = picks.map((p) =>
      p.id === move.id
        ? {
            id: p.id,
            pixel,
            matching,
            ink,
            alternatives,
            at: { x: move.x, y: move.y },
            ...(mismatch ? { mismatch } : {}),
          }
        : p,
    );
    if (move.done) status = `Moved to (${pixel.r}, ${pixel.g}, ${pixel.b}) → ${ink.name}`;
    if (recolored) requestRecolor();
  }

  // ── Material: the color the image is composited over ───────────────────────────────────────────
  // Live while the picker is open: inputs coalesce into the queued material change while it hasn't
  // started and nothing else was queued after it (`Serial.coalescing`), so a burst is one change
  // and never passes another action. Applying re-matches the picks on the new material: a pick
  // whose color changed gets the nearest ink again, the others keep theirs. Picks that now share a
  // color are kept.

  // `setMaterial` is called on every input event of the color picker; the coalescer turns a burst
  // of them into one queued change (see `Serial.coalescing` in `lib/serial.ts`).
  const queueMaterial = mutations.coalescing(applyMaterial);

  function setMaterial(color: Rgb) {
    materialInput = hex(color);
    queueMaterial({ color, input: ++materialInputs });
  }

  /** Back to "none": the picks are re-matched on white, the material's own entry goes. */
  function resetMaterial() {
    materialInput = hex(WHITE);
    queueMaterial({ color: WHITE, input: ++materialInputs, reset: true });
  }

  // The parameter is an object, unpacked in place: `color` is renamed to `next`, and `reset`
  // defaults to `false` when absent.
  /** Applies material input number `input` (runs inside `mutations`); `reset`: back to "none". */
  async function applyMaterial({
    color: next,
    input,
    reset = false,
  }: {
    color: Rgb;
    input: number;
    reset?: boolean;
  }) {
    const changed = !sameRgb(next, material);
    let resuggested = 0;
    if (changed) {
      const gen = generation;
      const rematched = await rematch(picks, next);
      if (!rematched) {
        // Not applied: the input goes back to the applied material, unless it shows a newer choice.
        if (input === materialInputs) materialInput = hex(material);
        return;
      }
      // A new image opened meanwhile has no picks yet (its picks queue behind this task).
      if (gen === generation) {
        // Count the picks whose matching color changed (and so got a new suggested ink).
        resuggested = rematched.filter((p, i) => !sameRgb(p.matching, picks[i]!.matching)).length;
        picks = rematched;
      }
      material = next;
    }
    const changes = resuggested
      ? ` · ${resuggested} pick${resuggested === 1 ? '' : 's'} re-suggested`
      : '';
    if (reset) {
      materialChosen = false;
      for (const r of ranges) if (r.material) deltaQueues.delete(r.id);
      ranges = ranges.filter((r) => !r.material);
      status = `Material reset${changes}.`;
    } else {
      // Choosing a material (even the one in use) leaves its own color unprinted, if there's room.
      materialChosen = true;
      const full = !setMaterialRange(next)
        ? ` · its own color is printed: at most ${LIMITS.unprinted} colors can be left unprinted`
        : '';
      status = `Material ${hex(next)}${changes}${full}.`;
    }
    requestRecolor();
    if (changed && image) void countColors();
  }

  /**
   * The material's own color as a range with the default ΔE: added if missing (also after being
   * removed), otherwise moved to the new color with its ΔE kept. Not added when the list is full
   * (`LIMITS.unprinted`). Returns whether the material's color is left unprinted.
   */
  function setMaterialRange(color: Rgb): boolean {
    const pixel = { ...color, a: 255 };
    if (ranges.some((r) => r.material)) {
      ranges = ranges.map((r) => (r.material ? { ...r, pixel } : r));
      return true;
    }
    if (ranges.length >= LIMITS.unprinted) return false;
    ranges = [
      {
        id: nextRangeId++,
        pixel,
        deltaE: RANGE_DELTA_E,
        maxDeltaE: RANGE_SLIDER_MAX,
        material: true,
      },
      ...ranges,
    ];
    return true;
  }

  /**
   * The picks re-matched on `on`, in order: a pick whose composited color changed gets the
   * nearest ink and alternatives, as a fresh click would; the others are returned unchanged.
   * `undefined` if the engine call failed (the error is shown).
   */
  async function rematch(list: PickEntry[], on: Rgb): Promise<PickEntry[] | undefined> {
    if (list.length === 0) return [];
    // Svelte state is wrapped in proxies, which can't be sent to the worker; send a plain copy.
    const plain = $state.snapshot(list);
    const outcome = await client.call((api) =>
      api.rematch(
        plain.map((p) => ({ pixel: p.pixel, matching: p.matching })),
        on,
      ),
    );
    if (outcome.status === 'error') {
      if (outcome.error.kind !== 'superseded') showError(outcome.error.kind, outcome.error.message);
      return undefined;
    }
    return list.map((p, i) => {
      const r = outcome.value[i];
      const ink = r?.alternatives?.[0];
      // `{ ...p, field: value }` copies the pick and overrides some fields; the original object
      // isn't changed.
      return r && ink ? { ...p, matching: r.matching, ink, alternatives: r.alternatives! } : p;
    });
  }

  // ── Colors left unprinted: no ink there, the material shows instead ────────────────────────────
  // A range is a pixel from the original and a ΔE: every pixel whose color (composited over the
  // material, in Rust) is within that ΔE of the range's pixel takes no ink and is transparent in
  // the result, so the material shows through. Ranges are checked before the picks.

  /** Adds a material range at the clicked pixel (queued like a pick: it reads the material). */
  function addRange(x: number, y: number, seen: Rgba | undefined) {
    const gen = generation;
    void mutations.run(async () => {
      if (gen !== generation) return;
      if (ranges.length >= LIMITS.unprinted) {
        status = `At most ${LIMITS.unprinted} colors can be left unprinted.`;
        return;
      }
      const on = material;
      const picked = await client.call((api) => api.pick(gen, x, y, on, seen));
      if (gen !== generation) return;
      if (picked.status === 'error') {
        showError(picked.error.kind, picked.error.message);
        return;
      }
      const { pixel } = picked.value;
      ranges = [
        ...ranges,
        {
          id: nextRangeId++,
          pixel,
          deltaE: RANGE_DELTA_E,
          maxDeltaE: RANGE_SLIDER_MAX,
          at: { x, y },
        },
      ];
      status = `Left to the material: (${pixel.r}, ${pixel.g}, ${pixel.b}) and colors within ΔE ${RANGE_DELTA_E}.`;
      requestRecolor();
    });
  }

  // Entry changes are queued like pick changes, so they keep their place among material changes
  // and imports. A slider drag is one change per burst: one coalescer per entry.
  // `SvelteMap` is a `Map` whose changes Svelte can track (a plain `Map` would not be reactive).
  const deltaQueues = new SvelteMap<number, (deltaE: number) => void>();

  function setRangeDeltaE(id: number, deltaE: number) {
    let push = deltaQueues.get(id);
    if (!push) {
      push = mutations.coalescing<number>((value) => {
        ranges = ranges.map((r) => (r.id === id ? { ...r, deltaE: value } : r));
        requestRecolor();
      });
      deltaQueues.set(id, push);
    }
    push(deltaE);
  }

  function removeRange(id: number) {
    void mutations.run(() => {
      ranges = ranges.filter((r) => r.id !== id);
      deltaQueues.delete(id);
      requestRecolor();
    });
  }

  /** Queues a synchronous pick-list change behind any pending one. */
  function mutate(change: () => void) {
    const gen = generation;
    void mutations.run(() => {
      if (gen !== generation) return;
      change();
      requestRecolor();
    });
  }

  const changeInk = (id: number, index: number) =>
    mutate(() => {
      picks = picks.map((p) =>
        p.id === id && p.alternatives[index] ? { ...p, ink: p.alternatives[index] } : p,
      );
    });

  const removePick = (id: number) =>
    mutate(() => {
      picks = picks.filter((p) => p.id !== id);
      track('Pick removed', { picks: picks.length });
    });

  const clearPicks = () =>
    mutate(() => {
      track('Picks cleared', { removed: picks.length });
      picks = [];
    });

  // ── Palette configs: import replaces the picks only after everything validated ─────────────────
  let configInput: HTMLInputElement;
  let sizeDialog: HTMLDialogElement;
  let configSections = $state<ConfigSection[]>([]);
  let configName = $state('');
  /** A config's material and unprinted colors (for every section), while its dialog is open. */
  interface ConfigLook {
    material: Rgb;
    unprinted: ConfigUnprinted[];
  }
  let configLook: ConfigLook = { material: WHITE, unprinted: [] };

  // A config file was chosen. `files?.[0]` is the first chosen file, if any; resetting the input's
  // value lets the same file be chosen again later.
  function onconfigchosen(e: Event & { currentTarget: HTMLInputElement }) {
    const f = e.currentTarget.files?.[0];
    e.currentTarget.value = '';
    if (!f) return;
    if (f.size > LIMITS.configBytes) {
      showError(
        'fileTooLarge',
        `${f.name}: the config is larger than ${LIMITS.configBytes / 1024} KB.`,
      );
      return;
    }
    const gen = generation;
    const inputs = materialInputs;
    // Queued like a pick, so imports and picks apply in the order they were made.
    void mutations.run(async () => {
      const parsed = await client.call(async (api) => api.parseConfig(await f.text()));
      if (gen !== generation) return;
      if (parsed.status === 'error') {
        showError(parsed.error.kind, `${f.name}: ${parsed.error.message}`);
        return;
      }
      const { sections, material: fileMaterial, unprinted } = parsed.value;
      if (sections.length === 0) {
        showError('invalidConfig', `${f.name}: the config has no palettes.`);
      } else if (sections.length === 1) {
        await applySection(
          sections[0]!,
          { material: fileMaterial, unprinted },
          f.name,
          gen,
          inputs,
        );
      } else {
        // The choice in the dialog is the next action; it queues the import then.
        configSections = sections;
        configName = f.name;
        configLook = { material: fileMaterial, unprinted };
        // Opens the `<dialog>` (in the markup below) as a modal: the rest of the page is inert
        // until it closes.
        sizeDialog.showModal();
      }
    });
  }

  function chooseSection(section: ConfigSection) {
    sizeDialog.close();
    const gen = generation;
    // Svelte state is a proxy, which can't be posted to the worker: keep a plain copy.
    const plain = $state.snapshot(section);
    const name = configName;
    const look = configLook;
    const inputs = materialInputs;
    void mutations.run(() => applySection(plain, look, name, gen, inputs));
  }

  /**
   * Resolves a section on the config's material and replaces the picks, the material and the
   * unprinted colors in one step (runs inside `mutations`): the file is the whole state.
   * Duplicates in the file are kept, as written. `inputs`: the material inputs made before
   * the import was chosen.
   */
  async function applySection(
    section: ConfigSection,
    { material: fileMaterial, unprinted }: ConfigLook,
    name: string,
    gen: number,
    inputs: number,
  ) {
    if (gen !== generation) return;
    const resolved = await client.call((api) => api.resolveSection(section, fileMaterial));
    if (gen !== generation) return;
    if (resolved.status === 'error') {
      showError(resolved.error.kind, `${name}: ${resolved.error.message}`);
      return;
    }
    picks = resolved.value.map((p) => ({
      id: nextPickId++,
      pixel: p.pixel,
      matching: p.matching,
      ink: p.ink,
      alternatives: p.alternatives,
    }));
    const materialChanged = !sameRgb(fileMaterial, material);
    material = fileMaterial;
    // Imported entries have no square: like imported picks, they weren't clicked.
    // A value above the slider's usual range widens that entry's slider.
    const maxDeltaE = (deltaE: number) => (deltaE > RANGE_SLIDER_MAX ? 100 : RANGE_SLIDER_MAX);
    ranges = unprinted.map((u) =>
      u.kind === 'material'
        ? {
            id: nextRangeId++,
            pixel: { ...fileMaterial, a: 255 },
            deltaE: u.deltaE,
            maxDeltaE: maxDeltaE(u.deltaE),
            material: true,
          }
        : { id: nextRangeId++, pixel: u.rgba, deltaE: u.deltaE, maxDeltaE: maxDeltaE(u.deltaE) },
    );
    // The file decides, "none" included; a white material can't tell a choice from the default
    // unless its own color is listed.
    materialChosen = !sameRgb(fileMaterial, WHITE) || unprinted.some((u) => u.kind === 'material');
    // A material chosen after the import is queued behind it and already shown.
    if (inputs === materialInputs) materialInput = hex(fileMaterial);
    const left = unprinted.length
      ? `, ${unprinted.length} color${unprinted.length === 1 ? '' : 's'} left unprinted`
      : '';
    status = `Imported ${picks.length} pick${picks.length === 1 ? '' : 's'} (size ${section.size}) from ${name}${materialChanged ? `, on material ${hex(fileMaterial)}` : ''}${left}.`;
    track('Picks imported', {
      size: section.size,
      picks: picks.length,
      unprinted: unprinted.length,
    });
    requestRecolor();
    if (materialChanged) void countColors();
  }

  // Builds the config text in the worker (the same writer as the CLI's) and offers it as a
  // download.
  async function exportPicks() {
    if (!file) return;
    const name = file.name;
    const on = material;
    const exported = await client.call((api) =>
      api.exportConfig(
        name,
        on,
        picks.map((p) => ({ rgba: $state.snapshot(p.pixel), ink: p.ink.name })),
        ranges.map((r): ConfigUnprinted =>
          r.material
            ? { kind: 'material', deltaE: r.deltaE }
            : { kind: 'color', rgba: $state.snapshot(r.pixel), deltaE: r.deltaE },
        ),
      ),
    );
    if (exported.status === 'error') {
      showError(exported.error.kind, exported.error.message);
      return;
    }
    save(new Blob([exported.value], { type: 'application/toml' }), `${stem(name)}.palettes.toml`);
    track('Picks exported', { picks: picks.length, unprinted: ranges.length });
  }

  // ── Live recolor: one in flight, one pending; stale results dropped ────────────────────────────
  interface RecolorRequest {
    generation: number;
    revision: number;
    mappings: ReturnType<typeof mappings>;
    /** The material these mappings were made on: a result never mixes two materials. */
    material: Rgb;
    materialRanges: { pixel: Rgba; deltaE: number }[];
  }

  // At most one recolor runs at a time; while it runs, only the newest request waits (see
  // `lib/latest.ts`). A result is shown only if it is for the current image and revision.
  const recolorer = new Latest<RecolorRequest>(async (req) => {
    recoloring = true;
    const outcome = await client.call((api) =>
      api.recolor(req.generation, req.revision, req.mappings, req.material, req.materialRanges),
    );
    const current = req.generation === generation && req.revision === revision;
    recoloring = recolorer.busy && !current;
    if (outcome.status === 'error') {
      if (outcome.error.kind !== 'superseded' && req.generation === generation) {
        showError(outcome.error.kind, outcome.error.message);
      }
      return;
    }
    const { bitmap } = outcome.value;
    if (current) {
      result?.close();
      result = bitmap;
      resultRevision = req.revision;
      recoloring = false;
    } else {
      bitmap.close();
    }
  });

  // Called after every change that affects the result. Each call is a new revision; the request
  // carries plain copies of the picks, the material and the unprinted colors.
  function requestRecolor() {
    if (!image) return;
    revision++;
    if (picks.length === 0) {
      // Nothing is printed before the first pick; the preview shows only the material (or the
      // checkerboard), and there is nothing to download. Older results are dropped (revision).
      result?.close();
      result = undefined;
      resultRevision = -1;
      unprintedPicks = [];
      return;
    }
    const materialRanges = ranges.map((r) => ({
      pixel: $state.snapshot(r.pixel),
      deltaE: r.deltaE,
    }));
    recolorer.request({
      generation,
      revision,
      mappings: $state.snapshot(mappings(picks)),
      material,
      materialRanges,
    });
    void checkUnprinted(revision, materialRanges);
  }

  /**
   * Finds the picks whose own color the unprinted colors cover (recolor's own test, in the
   * worker), for the note in the pick list. Only the answer for the current revision is kept.
   */
  async function checkUnprinted(rev: number, materialRanges: { pixel: Rgba; deltaE: number }[]) {
    if (materialRanges.length === 0) {
      unprintedPicks = [];
      return;
    }
    const ids = picks.map((p) => p.id);
    const colors = picks.map((p) => $state.snapshot(p.matching));
    const on = material;
    const outcome = await client.call((api) => api.unprintedColors(colors, on, materialRanges));
    if (rev !== revision) return;
    unprintedPicks = outcome.status === 'ok' ? ids.filter((_, i) => outcome.value[i]) : [];
  }

  // ── Download: always the current revision at full resolution ───────────────────────────────────
  async function download() {
    if (!file) return;
    // The name and image at the moment of the click; nothing is saved if either changes.
    const gen = generation;
    const name = file.name;
    await recolorer.idle();
    const rev = revision;
    if (gen !== generation || resultRevision !== rev) return;
    const png: AppOutcome<Blob> = await client.call((api) => api.encodePng(gen, rev));
    if (gen !== generation || rev !== revision) return; // changed while encoding
    if (png.status === 'error') {
      if (png.error.kind !== 'superseded') showError(png.error.kind, png.error.message);
      return;
    }
    save(png.value, `${stem(name)}-rekolor.png`);
    // `inks` counts distinct inks: several picks may print with the same one.
    track('PNG downloaded', {
      picks: picks.length,
      inks: new Set(picks.map((p) => p.ink.name)).size,
    });
  }

  // The file name without its extension: the regular expression matches a final `.` and what
  // follows it.
  const stem = (name: string) => name.replace(/\.[^.]+$/, '');

  // Downloads a file: a temporary object URL for the data, clicked through an invisible link. The
  // URL is released a little later, once the browser has started the download.
  function save(blob: Blob, name: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 10_000);
  }

  // ── Zoom: one view for both canvases ───────────────────────────────────────────────────────────
  // The views report their size: keep the same image point at the center when it changes (or fit
  // the image, the first time).
  function setViewport(size: Size) {
    if (size.width === 0 || size.height === 0) return;
    const before = viewport;
    viewport = size;
    if (!image) return;
    view = before.width && before.height ? resize(view, before, size) : fit(image, size);
  }

  const zoomBy = (factor: number) => {
    view = zoomAt(view, factor, viewport.width / 2, viewport.height / 2);
  };
  const zoomFit = () => image && (view = fit(image, viewport));
  const zoomActual = () => {
    if (!image) return;
    view = zoomAt(view, 1 / view.scale, viewport.width / 2, viewport.height / 2);
  };

  // ── Files: chooser, drag and drop, paste ───────────────────────────────────────────────────────
  let fileInput: HTMLInputElement;
  let dragging = $state(false);

  function onchosen(e: Event & { currentTarget: HTMLInputElement }) {
    const f = e.currentTarget.files?.[0];
    if (f) void openFile(f);
    e.currentTarget.value = '';
  }

  // `preventDefault()` stops the browser from opening the dropped file itself.
  function ondrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const f = e.dataTransfer?.files[0];
    if (f) void openFile(f);
  }

  function onpaste(e: ClipboardEvent) {
    const f = [...(e.clipboardData?.files ?? [])].find((x) => x.type.startsWith('image/'));
    if (f) {
      e.preventDefault();
      void openFile(f);
    }
  }

  // Runs when the component is removed: free the bitmaps and stop the worker.
  onDestroy(() => {
    original?.close();
    result?.close();
    client.dispose();
  });

  // `[string, string][]`: a list of pairs (letter, color).
  /** The title in the 2023 app's rainbow colors. */
  const rainbow: [string, string][] = [
    ['r', '#e81416'],
    ['e', '#ffa500'],
    ['K', '#faeb36'],
    ['o', '#79c314'],
    ['l', '#487de7'],
    ['o', '#4b369d'],
    ['r', '#70369d'],
  ];

  const megabytes = (n: number) =>
    n < 1024 * 1024 ? `${(n / 1024).toFixed(0)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;
</script>

<!-- Window-wide handlers: paste an image anywhere, or drag one onto the page. `{onpaste}` is
     short for `onpaste={onpaste}`. -->
<svelte:window
  {onpaste}
  ondragover={(e) => {
    e.preventDefault();
    dragging = true;
  }}
  ondragleave={(e) => {
    if (!e.relatedTarget) dragging = false;
  }}
  {ondrop}
/>

<header class="top">
  <!-- The 2023 rainbow, one color per letter; read as one word by screen readers. -->
  <h1 aria-label="reKolor">
    {#each rainbow as [letter, color], i (i)}<span aria-hidden="true" style:color>{letter}</span
      >{/each}
  </h1>
  <p class="tagline">Preview a picture printed with a limited ink palette.</p>
  <BuiltWith />
  <!-- What the published site records, and why: a separate page (`public/privacy.html`), in a new
       tab so the open image isn't lost. -->
  <a class="privacy" href={privacyUrl} target="_blank" rel="noopener noreferrer">Privacy</a>
</header>

<!-- `class:dragging` adds the `dragging` class while the `dragging` variable is true (the
     dashed outline). -->
<main class:dragging>
  <section class="toolbar" aria-label="Image and zoom">
    <input
      bind:this={fileInput}
      id="file"
      type="file"
      data-private
      accept="image/*"
      class="visually-hidden"
      tabindex="-1"
      aria-hidden="true"
      onchange={onchosen}
      disabled={!ready}
    />
    <!-- The real file input is hidden; this button opens its file chooser. -->
    <button type="button" class="primary" onclick={() => fileInput.click()} disabled={!ready}>
      Open image…
    </button>
    <!-- `data-private`: LogRocket never records this element or what is in it, here the image's
         file name (see `lib/logrocket.ts`). The status line, the error line and the palette
         dialog below are private for the same reason. -->
    <span class="file-info" data-testid="file-info" data-private>
      {#if file && image}
        {file.name} · {megabytes(file.size)} · {image.width} × {image.height}
        {#if colors !== undefined}· {colors.toLocaleString()} colors{:else}· counting colors…{/if}
      {:else}
        or drop / paste an image
      {/if}
    </span>
    <span class="spacer"></span>
    <div class="zoom" role="group" aria-label="Zoom">
      <button type="button" onclick={() => zoomBy(1 / 1.5)} disabled={!image} aria-label="Zoom out"
        >−</button
      >
      <button type="button" onclick={zoomFit} disabled={!image}>Fit</button>
      <button type="button" onclick={zoomActual} disabled={!image}>1:1</button>
      <button type="button" onclick={() => zoomBy(1.5)} disabled={!image} aria-label="Zoom in"
        >+</button
      >
      <output aria-label="Zoom level">{image ? `${Math.round(view.scale * 100)} %` : ''}</output>
    </div>
    <button
      type="button"
      onclick={download}
      disabled={!image || resultRevision < 0}
      data-testid="download">Download PNG</button
    >
  </section>

  <section class="views">
    <!-- The two views share one `view` (zoom and pan), so they always show the same part of the
         image. `{view}` is short for `view={view}`. The marker of a pick being dragged is drawn
         where the drag is (`moving`), not where the pick still is. -->
    <ImageView
      label={addingRange
        ? 'Original — click a color to leave it unprinted (the material shows)'
        : 'Original — click to pick a color, Shift-drag a circle to move it'}
      bitmap={original}
      {view}
      markers={picks.flatMap((p) =>
        moving?.id === p.id ? [moving] : p.at ? [{ id: p.id, ...p.at }] : [],
      )}
      squares={ranges.flatMap((r) => (r.at ? [r.at] : []))}
      placeholder={opening
        ? 'Opening…'
        : 'Open, drop or paste an image (PNG, JPEG, WebP, GIF, AVIF…)'}
      onview={(v) => (view = v)}
      onviewport={setViewport}
      onpick={pickAt}
      onmove={(move) => mover.update(move)}
      onmovecancel={(drag) => mover.cancel(drag)}
    />
    <ImageView
      label={!image
        ? 'Preview'
        : picks.length === 0
          ? 'No picks yet: nothing is printed'
          : `Printed with ${picks.length} ink${picks.length === 1 ? '' : 's'}${ranges.length ? ` · ${ranges.length} color${ranges.length === 1 ? '' : 's'} left to the material` : ''}${recoloring ? ' · updating…' : ''}`}
      bitmap={result}
      {view}
      placeholder={!image ? 'The preview appears here.' : picks.length === 0 ? '' : 'Recoloring…'}
      onview={(v) => (view = v)}
      onviewport={() => {}}
      backdrop={materialChosen ? css(material) : undefined}
    />
  </section>

  <p class="status" role="status" data-testid="status" data-private>{status}</p>
  {#if error}
    <p class="error" role="alert" data-private>
      {error}
      <button type="button" class="link" onclick={() => (error = undefined)}>Dismiss</button>
    </p>
  {/if}

  <!-- Shown by `sizeDialog.showModal()` when an imported config has several palettes. -->
  <dialog bind:this={sizeDialog} aria-labelledby="size-title">
    <h2 id="size-title">Which palette?</h2>
    <p data-private>{configName} has several palettes. Import one as the pick list:</p>
    <div class="sizes">
      {#each configSections as section (section.size)}
        <button type="button" onclick={() => chooseSection(section)}>
          Size {section.size} · {section.picks.length} pick{section.picks.length === 1 ? '' : 's'}
        </button>
      {/each}
    </div>
    <button type="button" class="link" onclick={() => sizeDialog.close()}>Cancel</button>
  </dialog>

  <div class="bottom">
    <section class="picks-section" aria-labelledby="picks-title">
      <div class="picks-header">
        <h2 id="picks-title">Picks</h2>
        <input
          bind:this={configInput}
          type="file"
          data-private
          accept=".toml,application/toml,text/plain"
          class="visually-hidden"
          tabindex="-1"
          aria-hidden="true"
          onchange={onconfigchosen}
        />
        <button type="button" onclick={() => configInput.click()} disabled={!image}>
          Import picks…
        </button>
        <button type="button" onclick={exportPicks} disabled={!image || picks.length === 0}>
          Export picks
        </button>
        {#if picks.length > 0}
          <button type="button" class="link" onclick={clearPicks}>Clear all</button>
        {/if}
      </div>
      <!-- The pick list (`components/PickList.svelte`): it shows the picks and reports changes
           through `onink` and `onremove`. -->
      <PickList
        {picks}
        {material}
        unprinted={unprintedPicks}
        onink={changeInk}
        onremove={removePick}
      />
    </section>

    <section class="material-section" aria-labelledby="material-title">
      <h2 id="material-title">Material</h2>
      <div class="material">
        <label title="The garment or surface color the image is printed on">
          Color
          <!-- The native color picker; `oninput` fires continuously while the user drags in it. -->
          <span class="material-swatch" class:none={!materialChosen}>
            <input
              type="color"
              value={materialInput}
              oninput={(e) => setMaterial(fromHex(e.currentTarget.value))}
              data-testid="material"
            />
          </span>
          {#if !materialChosen}<span class="muted">none</span>{/if}
        </label>
        <button
          type="button"
          onclick={() => setMaterial(WHITE)}
          disabled={materialChosen && materialInput === hex(WHITE)}>White</button
        >
        <button
          type="button"
          onclick={() => setMaterial(BLACK)}
          disabled={materialChosen && materialInput === hex(BLACK)}>Black</button
        >
        <button type="button" onclick={resetMaterial} disabled={!materialChosen}>Reset</button>
      </div>

      <div class="unprinted-header">
        <h3>Not printed <span class="muted">(the material shows)</span></h3>
        <button
          type="button"
          class="add-range"
          aria-pressed={addingRange}
          aria-label="Leave a color unprinted"
          title={addingRange
            ? 'Click a color in the original (click + again to cancel)'
            : 'Leave a color unprinted: click it in the original'}
          disabled={!image}
          onclick={() => (addingRange = !addingRange)}>+</button
        >
      </div>
      {#if ranges.length === 0}
        <p class="hint">
          Colors the material already has don't need ink. Pick one in the original; it and the
          colors within its ΔE show the material instead (□ on the original).
        </p>
      {:else}
        <ol class="ranges" aria-label="Colors left to the material">
          {#each ranges as range (range.id)}
            <!-- Each unprinted color: its swatch over the material, a remove button and a ΔE
                 slider. -->
            {@const label = `rgb ${range.pixel.r}, ${range.pixel.g}, ${range.pixel.b}${range.pixel.a < 255 ? `, alpha ${range.pixel.a}` : ''}`}
            <li class="range">
              <span class="range-swatch" style:background={css(material)} title={label}>
                <span style:background={cssAlpha(range.pixel)}></span>
              </span>
              <span class="range-color"
                >{range.material ? `Material color · ${hex(range.pixel)}` : label}</span
              >
              <button
                type="button"
                class="remove"
                aria-label="Print {label} again"
                onclick={() => removeRange(range.id)}>×</button
              >
              <label class="range-delta">
                ΔE
                <input
                  type="range"
                  min="0"
                  max={range.maxDeltaE}
                  step="1"
                  value={range.deltaE}
                  oninput={(e) => setRangeDeltaE(range.id, Number(e.currentTarget.value))}
                />
                <output>{range.deltaE}</output>
              </label>
            </li>
          {/each}
        </ol>
      {/if}
    </section>
  </div>
</main>

<style>
  /* These styles apply only to this component's own elements (Svelte adds a unique class to
     scope them). `clamp(min, preferred, max)` lets the side padding grow with the window
     width. */
  .top {
    display: flex;
    align-items: baseline;
    gap: 1rem;
    flex-wrap: wrap;
    padding: 1rem clamp(1rem, 3vw, 2rem) 0;
  }
  h1 {
    margin: 0;
    font-size: 1.6rem;
    letter-spacing: 0.02em;
  }
  .tagline {
    margin: 0;
    color: var(--muted);
  }
  .privacy {
    color: var(--muted);
  }
  main {
    padding: 1rem clamp(1rem, 3vw, 2rem) 2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  /* Shown while a file is dragged over the page. */
  main.dragging {
    outline: 3px dashed var(--accent);
    outline-offset: -6px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .file-info {
    color: var(--muted);
    font-size: 0.9rem;
  }
  .spacer {
    flex: 1;
  }
  .zoom {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  /* Bottom: picks on the left (as many columns as fit), material on the right (one column). */
  .bottom {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(16rem, 22rem);
    gap: 1rem 2rem;
    align-items: start;
  }
  @media (max-width: 800px) {
    .bottom {
      grid-template-columns: 1fr;
    }
  }
  .material-section {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.6rem;
  }
  .material-section h2 {
    margin: 0;
  }
  h3 {
    font-size: 1rem;
    margin: 0;
  }
  .unprinted-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-top: 0.6rem;
  }
  .add-range {
    font-size: 1.2rem;
    line-height: 1;
    width: 2rem;
    height: 2rem;
    padding: 0;
  }
  .muted,
  .hint {
    color: var(--muted);
    font-weight: normal;
  }
  .hint {
    margin: 0;
    font-size: 0.9rem;
  }
  .material,
  .material label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .material label {
    color: var(--muted);
    font-size: 0.9rem;
  }
  .add-range[aria-pressed='true'] {
    outline: 2px solid var(--accent);
  }
  .ranges {
    list-style: none;
    margin: 0;
    padding: 0;
    align-self: stretch;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    font-size: 0.9rem;
  }
  .range {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.4rem 0.75rem;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
  }
  .range-delta {
    grid-column: 2 / -1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--muted);
  }
  .range-delta input {
    flex: 1;
    min-width: 0;
  }
  .range-delta output {
    min-width: 2ch;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--fg);
  }
  .range .remove {
    font-size: 1.3rem;
    line-height: 1;
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .range .remove:hover {
    border-color: var(--border);
    color: var(--fg);
  }
  .range-swatch {
    grid-row: span 2;
    display: inline-block;
    width: 2rem;
    height: 2rem;
    border: 1px solid var(--border);
    border-radius: 2px;
    overflow: hidden;
  }
  .range-swatch > span {
    display: block;
    width: 100%;
    height: 100%;
  }
  .material-swatch {
    position: relative;
    display: inline-block;
    width: 2.2rem;
    height: 1.8rem;
    border-radius: 4px;
  }
  /* No material yet: a transparent (checkerboard) swatch; the picker still opens on click. */
  .material-swatch.none {
    border: 1px solid var(--border);
    background:
      repeating-conic-gradient(var(--checker) 0% 25%, transparent 0% 50%) 50% / 10px 10px,
      var(--surface);
  }
  .material-swatch.none input {
    opacity: 0;
  }
  .material input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: none;
    cursor: pointer;
  }
  .zoom output {
    min-width: 4.5ch;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
    font-size: 0.9rem;
  }
  .views {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 420px), 1fr));
    gap: 1rem;
  }
  .status {
    margin: 0;
    color: var(--muted);
    min-height: 1.4em;
  }
  .error {
    margin: 0;
    padding: 0.6rem 0.8rem;
    border-radius: 6px;
    background: var(--error-bg);
    color: var(--error);
  }
  .picks-header {
    display: flex;
    align-items: baseline;
    gap: 1rem;
  }
  h2 {
    font-size: 1.1rem;
    margin: 0 0 0.5rem;
  }
  .picks-header {
    flex-wrap: wrap;
  }
  dialog {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--fg);
    max-width: min(90vw, 28rem);
  }
  /* `::backdrop` is the layer behind a modal dialog, covering the page. */
  dialog::backdrop {
    background: rgb(0 0 0 / 0.4);
  }
  .sizes {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }
</style>
