<script lang="ts">
  import type { ConfigSection, Rgb, Rgba } from 'rekolor-wasm';
  import { onDestroy } from 'svelte';
  import ImageView from './components/ImageView.svelte';
  import PickList from './components/PickList.svelte';
  import { EngineClient } from './lib/client';
  import { ALTERNATIVES } from './lib/engine';
  import { Latest } from './lib/latest';
  import { Mover, type Marker } from './lib/moves';
  import { LIMITS } from './lib/limits';
  import type { AppOutcome } from './lib/outcome';
  import {
    WHITE,
    css,
    fromHex,
    hex,
    mappings,
    sameRgb,
    type PickEntry,
    type PickMove,
  } from './lib/picks';
  import { Serial } from './lib/serial';
  import { fit, resize, zoomAt, type Size, type View } from './lib/view';

  // ── Engine ───────────────────────────────────────────────────────────────────────────────────
  let ready = $state(false);
  let status = $state('Starting the engine…');
  let error = $state<string | undefined>();

  const client = new EngineClient((reason) => {
    resetImage();
    error = `The engine stopped (${reason}) and was restarted. Open the image again.`;
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
      error = outcome.error.message;
    }
  }
  void start();

  // ── Image state ──────────────────────────────────────────────────────────────────────────────
  /** Incremented per open attempt (the worker requires each to be newer than the last). */
  let requested = 0;
  /**
   * The generation of the image that is open, set only when an open succeeds, so a failed
   * replacement keeps the previous image usable (review fix). Results for others are ignored.
   */
  let generation = 0;
  /** Incremented per pick change; only the current revision's result is shown or downloaded. */
  let revision = 0;
  let file = $state<{ name: string; size: number } | undefined>();
  let image = $state<Size | undefined>();
  let colors = $state<number | undefined>();
  let original = $state.raw<ImageBitmap | undefined>();
  let result = $state.raw<ImageBitmap | undefined>();
  let resultRevision = $state(-1);
  let opening = $state(false);
  let recoloring = $state(false);
  let picks = $state<PickEntry[]>([]);
  let nextPickId = 1;
  /** Pick-list changes run one at a time, in the order of the user's actions (review fix). */
  const mutations = new Serial();
  /** The pick being Shift-dragged: its marker follows the pointer until the move settles. */
  let moving = $state<Marker>();
  /**
   * The material color (the garment or substrate) every engine call composites over. Changed only
   * inside a `mutations` task, together with the picks re-matched on it, so the two always agree.
   * Kept across new images and worker restarts.
   */
  let material = $state.raw<Rgb>(WHITE);
  /** What the color input shows: the user's newest choice, maybe not applied yet. */
  let materialInput = $state(hex(WHITE));
  /**
   * Material inputs so far. An action (a material change, an import) writes `materialInput` only if
   * no input came after it, so the input never jumps back from a newer choice.
   */
  let materialInputs = 0;

  let view = $state<View>({ scale: 1, x: 0, y: 0 });
  let viewport: Size = { width: 0, height: 0 };

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
    if (gen !== requested) {
      // A newer open was started meanwhile (or the worker restarted); it owns `opening`.
      if (opened.status === 'ok') opened.value.bitmap.close();
      return;
    }
    opening = false;
    if (opened.status === 'error') {
      if (opened.error.kind !== 'superseded') {
        error = `${f.name}: ${opened.error.message}`;
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
    mover.reset();
    view = fit(image, viewport);
    status = 'Click a color in the original to pick it.';
    requestRecolor();
    await countColors();
  }

  /** Counts the current image's colors on the current material (shown in the toolbar). */
  async function countColors() {
    const gen = generation;
    const on = material;
    colors = undefined;
    const counted = await client.call((api) => api.colorCount(gen, on));
    // Only for the image and material it was made for.
    if (gen === generation && on === material && counted.status === 'ok') colors = counted.value;
  }

  // ── Picks ────────────────────────────────────────────────────────────────────────────────────
  function pickAt(x: number, y: number, seen: Rgba | undefined) {
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
        error = picked.error.message;
        return;
      }
      const { pixel, matching, suggestion, mismatch } = picked.value;
      if (picks.some((p) => sameRgb(p.matching, matching))) {
        status = `That color (${pixel.r}, ${pixel.g}, ${pixel.b}) is already picked.`;
        return;
      }
      const nearest = await client.call((api) => api.nearest(matching, ALTERNATIVES));
      if (gen !== generation) return;
      const alternatives = nearest.status === 'ok' ? nearest.value : [suggestion];
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
    });
  }

  // ── Moving a pick (Shift-drag on its marker) ───────────────────────────────────────────────────
  // Live: each new pixel re-picks the color and re-suggests the ink, like a click there, and the
  // preview follows; within the same color the ink is kept (C1 a). A pixel whose color another pick
  // already has is skipped; on release the pick stays at its last valid pixel (F2 a). Escape or a
  // lost pointer cancels: the pick is put back as it was (F1 a). Ordering and coalescing: `Mover`.

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
      if (picked.error.kind !== 'superseded') error = picked.error.message;
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

  // ── Material (material-color M1–M6, C1): the color the image is composited over ─────────────
  // Live while the picker is open: inputs coalesce into the queued material change while it hasn't
  // started and nothing else was queued after it (`Serial.coalescing`), so a burst is one change and
  // never passes another action. Applying re-matches the picks on the new material (M3 a): a pick
  // whose color changed gets the nearest ink again, the others keep theirs. Picks that now share a
  // color are kept (M3.1 b).

  const queueMaterial = mutations.coalescing(applyMaterial);

  function setMaterial(color: Rgb) {
    materialInput = hex(color);
    queueMaterial({ color, input: ++materialInputs });
  }

  /** Applies material input number `input` (runs inside `mutations`). */
  async function applyMaterial({ color: next, input }: { color: Rgb; input: number }) {
    if (sameRgb(next, material)) return;
    const gen = generation;
    const rematched = await rematch(picks, next);
    if (!rematched) {
      // Not applied: the input goes back to the applied material, unless it shows a newer choice.
      if (input === materialInputs) materialInput = hex(material);
      return;
    }
    // A new image opened meanwhile has no picks yet (its picks queue behind this task).
    let resuggested = 0;
    if (gen === generation) {
      resuggested = rematched.filter((p, i) => !sameRgb(p.matching, picks[i]!.matching)).length;
      picks = rematched;
    }
    material = next;
    status = `Material ${hex(next)}${resuggested ? ` · ${resuggested} pick${resuggested === 1 ? '' : 's'} re-suggested` : ''}.`;
    requestRecolor();
    if (image) void countColors();
  }

  /**
   * The picks re-matched on `on` (M3 a), in order: a pick whose composited color changed gets the
   * nearest ink and alternatives, as a fresh click would; the others are returned unchanged.
   * `undefined` if the engine call failed (the error is shown).
   */
  async function rematch(list: PickEntry[], on: Rgb): Promise<PickEntry[] | undefined> {
    if (list.length === 0) return [];
    const plain = $state.snapshot(list);
    const outcome = await client.call((api) =>
      api.rematch(
        plain.map((p) => ({ pixel: p.pixel, matching: p.matching })),
        on,
      ),
    );
    if (outcome.status === 'error') {
      if (outcome.error.kind !== 'superseded') error = outcome.error.message;
      return undefined;
    }
    return list.map((p, i) => {
      const r = outcome.value[i];
      const ink = r?.alternatives?.[0];
      return r && ink ? { ...p, matching: r.matching, ink, alternatives: r.alternatives! } : p;
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
    });

  const clearPicks = () =>
    mutate(() => {
      picks = [];
    });

  // ── Palette configs (W10 v): import replaces the picks only after everything validated ──────────
  let configInput: HTMLInputElement;
  let sizeDialog: HTMLDialogElement;
  let configSections = $state<ConfigSection[]>([]);
  let configName = $state('');
  let configMaterial: Rgb = WHITE;

  function onconfigchosen(e: Event & { currentTarget: HTMLInputElement }) {
    const f = e.currentTarget.files?.[0];
    e.currentTarget.value = '';
    if (!f) return;
    if (f.size > LIMITS.configBytes) {
      error = `${f.name}: the config is larger than ${LIMITS.configBytes / 1024} KB.`;
      return;
    }
    const gen = generation;
    const inputs = materialInputs;
    // Queued like a pick, so imports and picks apply in the order they were made.
    void mutations.run(async () => {
      const parsed = await client.call(async (api) => api.parseConfig(await f.text()));
      if (gen !== generation) return;
      if (parsed.status === 'error') {
        error = `${f.name}: ${parsed.error.message}`;
        return;
      }
      const { sections, material: fileMaterial } = parsed.value;
      if (sections.length === 0) {
        error = `${f.name}: the config has no palettes.`;
      } else if (sections.length === 1) {
        await applySection(sections[0]!, fileMaterial, f.name, gen, inputs);
      } else {
        // The choice in the dialog is the next action; it queues the import then.
        configSections = sections;
        configName = f.name;
        configMaterial = fileMaterial;
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
    const fileMaterial = configMaterial;
    const inputs = materialInputs;
    void mutations.run(() => applySection(plain, fileMaterial, name, gen, inputs));
  }

  /**
   * Resolves a section on the config's material and replaces the picks and the material in one
   * step (runs inside `mutations`). Duplicates in the file are kept, as written (C4 a). `inputs`:
   * the material inputs made before the import was chosen.
   */
  async function applySection(
    section: ConfigSection,
    fileMaterial: Rgb,
    name: string,
    gen: number,
    inputs: number,
  ) {
    if (gen !== generation) return;
    const resolved = await client.call((api) => api.resolveSection(section, fileMaterial));
    if (gen !== generation) return;
    if (resolved.status === 'error') {
      error = `${name}: ${resolved.error.message}`;
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
    // A material chosen after the import is queued behind it and already shown.
    if (inputs === materialInputs) materialInput = hex(fileMaterial);
    status = `Imported ${picks.length} pick${picks.length === 1 ? '' : 's'} (size ${section.size}) from ${name}${materialChanged ? `, on material ${hex(fileMaterial)}` : ''}.`;
    requestRecolor();
    if (materialChanged) void countColors();
  }

  async function exportPicks() {
    if (!file) return;
    const name = file.name;
    const on = material;
    const exported = await client.call((api) =>
      api.exportConfig(
        name,
        on,
        picks.map((p) => ({ rgba: $state.snapshot(p.pixel), ink: p.ink.name })),
      ),
    );
    if (exported.status === 'error') {
      error = exported.error.message;
      return;
    }
    save(new Blob([exported.value], { type: 'application/toml' }), `${stem(name)}.palettes.toml`);
  }

  // ── Live recolor (W10 vi a): one in flight, one pending; stale results dropped ─────────────────
  interface RecolorRequest {
    generation: number;
    revision: number;
    mappings: ReturnType<typeof mappings>;
    /** The material these mappings were made on: a result never mixes two materials (C2 a). */
    material: Rgb;
  }

  const recolorer = new Latest<RecolorRequest>(async (req) => {
    recoloring = true;
    const outcome = await client.call((api) =>
      api.recolor(req.generation, req.revision, req.mappings, req.material),
    );
    const current = req.generation === generation && req.revision === revision;
    recoloring = recolorer.busy && !current;
    if (outcome.status === 'error') {
      if (outcome.error.kind !== 'superseded' && req.generation === generation) {
        error = outcome.error.message;
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

  function requestRecolor() {
    if (!image) return;
    revision++;
    recolorer.request({
      generation,
      revision,
      mappings: $state.snapshot(mappings(picks)),
      material,
    });
  }

  // ── Download (W9 a): always the current revision at full resolution ───────────────────────────
  async function download() {
    if (!file) return;
    // The name and image at the moment of the click; nothing is saved if either changes.
    const gen = generation;
    const name = file.name;
    await recolorer.idle();
    const rev = revision;
    if (gen !== generation || resultRevision !== rev) return;
    const png: AppOutcome<Blob> = await client.call((api) => api.encodePng(gen, rev));
    if (gen !== generation || rev !== revision) return; // changed while encoding (review fix)
    if (png.status === 'error') {
      if (png.error.kind !== 'superseded') error = png.error.message;
      return;
    }
    save(png.value, `${stem(name)}-rekolor.png`);
  }

  const stem = (name: string) => name.replace(/\.[^.]+$/, '');

  function save(blob: Blob, name: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 10_000);
  }

  // ── Zoom (W10 iv a): one view for both canvases ──────────────────────────────────────────────
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

  // ── Files: chooser, drag and drop, paste (W10 iii) ───────────────────────────────────────────
  let fileInput: HTMLInputElement;
  let dragging = $state(false);

  function onchosen(e: Event & { currentTarget: HTMLInputElement }) {
    const f = e.currentTarget.files?.[0];
    if (f) void openFile(f);
    e.currentTarget.value = '';
  }

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

  onDestroy(() => {
    original?.close();
    result?.close();
    client.dispose();
  });

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

  /** How labels name the material. */
  const onMaterial = $derived(
    sameRgb(material, WHITE) ? 'white' : `the material (${hex(material)})`,
  );

  const megabytes = (n: number) =>
    n < 1024 * 1024 ? `${(n / 1024).toFixed(0)} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`;
</script>

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
</header>

<main class:dragging>
  <section class="toolbar" aria-label="Image, material and zoom">
    <input
      bind:this={fileInput}
      id="file"
      type="file"
      accept="image/*"
      class="visually-hidden"
      tabindex="-1"
      aria-hidden="true"
      onchange={onchosen}
      disabled={!ready}
    />
    <button type="button" class="primary" onclick={() => fileInput.click()} disabled={!ready}>
      Open image…
    </button>
    <span class="file-info" data-testid="file-info">
      {#if file && image}
        {file.name} · {megabytes(file.size)} · {image.width} × {image.height}
        {#if colors !== undefined}· {colors.toLocaleString()} colors{:else}· counting colors…{/if}
      {:else}
        or drop / paste an image
      {/if}
    </span>
    <span class="spacer"></span>
    <div class="material" role="group" aria-label="Material">
      <label title="The garment or surface color the image is printed on">
        Material
        <input
          type="color"
          value={materialInput}
          oninput={(e) => setMaterial(fromHex(e.currentTarget.value))}
          data-testid="material"
        />
      </label>
      <button
        type="button"
        onclick={() => setMaterial(WHITE)}
        disabled={materialInput === hex(WHITE)}>White</button
      >
    </div>
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
    <ImageView
      label="Original — click to pick a color, Shift-drag a circle to move it"
      bitmap={original}
      {view}
      markers={picks.flatMap((p) =>
        moving?.id === p.id ? [moving] : p.at ? [{ id: p.id, ...p.at }] : [],
      )}
      placeholder={opening
        ? 'Opening…'
        : 'Open, drop or paste an image (PNG, JPEG, WebP, GIF, AVIF…)'}
      onview={(v) => (view = v)}
      onviewport={setViewport}
      onpick={pickAt}
      onmove={(move) => mover.update(move)}
      onmovecancel={(drag) => mover.cancel(drag)}
      backdrop={css(material)}
    />
    <ImageView
      label={!image
        ? 'Preview'
        : picks.length === 0
          ? `No picks yet: the image as printed on ${onMaterial}`
          : `Printed with ${picks.length} ink${picks.length === 1 ? '' : 's'}${recoloring ? ' · updating…' : ''}`}
      bitmap={result}
      {view}
      placeholder={image ? 'Recoloring…' : 'The preview appears here.'}
      onview={(v) => (view = v)}
      onviewport={() => {}}
    />
  </section>

  <p class="status" role="status" data-testid="status">{status}</p>
  {#if error}
    <p class="error" role="alert">
      {error}
      <button type="button" class="link" onclick={() => (error = undefined)}>Dismiss</button>
    </p>
  {/if}

  <dialog bind:this={sizeDialog} aria-labelledby="size-title">
    <h2 id="size-title">Which palette?</h2>
    <p>{configName} has several palettes. Import one as the pick list:</p>
    <div class="sizes">
      {#each configSections as section (section.size)}
        <button type="button" onclick={() => chooseSection(section)}>
          Size {section.size} · {section.picks.length} pick{section.picks.length === 1 ? '' : 's'}
        </button>
      {/each}
    </div>
    <button type="button" class="link" onclick={() => sizeDialog.close()}>Cancel</button>
  </dialog>

  <section class="picks-section" aria-labelledby="picks-title">
    <div class="picks-header">
      <h2 id="picks-title">Picks</h2>
      <input
        bind:this={configInput}
        type="file"
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
    <PickList {picks} {material} onink={changeInk} onremove={removePick} />
  </section>
</main>

<style>
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
  main {
    padding: 1rem clamp(1rem, 3vw, 2rem) 2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
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
  .material input {
    width: 2.2rem;
    height: 1.8rem;
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
