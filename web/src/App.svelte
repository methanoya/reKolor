<script lang="ts">
  import type { ConfigSection, ImageStats, Rgba } from 'rekolor-wasm';
  import { onDestroy } from 'svelte';
  import ImageView from './components/ImageView.svelte';
  import PickList from './components/PickList.svelte';
  import { EngineClient } from './lib/client';
  import { ALTERNATIVES } from './lib/engine';
  import { Latest } from './lib/latest';
  import { LIMITS } from './lib/limits';
  import type { AppOutcome } from './lib/outcome';
  import { mappings, sameRgb, type PickEntry } from './lib/picks';
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
  /** Incremented per opened image; results for older generations are ignored. */
  let generation = 0;
  /** Incremented per pick change; only the current revision's result is shown or downloaded. */
  let revision = 0;
  let file = $state<{ name: string; size: number } | undefined>();
  let image = $state<Size | undefined>();
  let stats = $state<ImageStats | undefined>();
  let original = $state.raw<ImageBitmap | undefined>();
  let result = $state.raw<ImageBitmap | undefined>();
  let resultRevision = $state(-1);
  let opening = $state(false);
  let recoloring = $state(false);
  let picks = $state<PickEntry[]>([]);
  let nextPickId = 1;

  let view = $state<View>({ scale: 1, x: 0, y: 0 });
  let viewport: Size = { width: 0, height: 0 };

  function resetImage() {
    generation++;
    original?.close();
    result?.close();
    original = undefined;
    result = undefined;
    image = undefined;
    stats = undefined;
    file = undefined;
    picks = [];
    resultRevision = -1;
  }

  async function openFile(f: File) {
    if (!ready) return;
    const gen = ++generation;
    opening = true;
    error = undefined;
    status = `Opening ${f.name}…`;
    const opened = await client.call((api) => api.open(f, gen));
    if (gen !== generation) {
      // A newer image was opened meanwhile.
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
    original?.close();
    result?.close();
    original = opened.value.bitmap;
    result = undefined;
    resultRevision = -1;
    image = { width: opened.value.width, height: opened.value.height };
    file = { name: f.name, size: f.size };
    stats = undefined;
    picks = [];
    view = fit(image, viewport);
    status = 'Click a color in the original to pick it.';
    requestRecolor();
    const analyzed = await client.call((api) => api.analyze(gen));
    if (gen === generation && analyzed.status === 'ok') stats = analyzed.value;
  }

  // ── Picks ────────────────────────────────────────────────────────────────────────────────────
  async function pickAt(x: number, y: number, seen: Rgba | undefined) {
    const gen = generation;
    const picked = await client.call((api) => api.pick(gen, x, y, seen));
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
  }

  function changeInk(id: number, index: number) {
    picks = picks.map((p) =>
      p.id === id && p.alternatives[index] ? { ...p, ink: p.alternatives[index] } : p,
    );
    requestRecolor();
  }

  function removePick(id: number) {
    picks = picks.filter((p) => p.id !== id);
    requestRecolor();
  }

  function clearPicks() {
    picks = [];
    requestRecolor();
  }

  // ── Palette configs (W10 v): import replaces the picks only after everything validated ──────────
  let configInput: HTMLInputElement;
  let sizeDialog: HTMLDialogElement;
  let configSections = $state<ConfigSection[]>([]);
  let configName = $state('');

  async function onconfigchosen(e: Event & { currentTarget: HTMLInputElement }) {
    const f = e.currentTarget.files?.[0];
    e.currentTarget.value = '';
    if (!f) return;
    if (f.size > LIMITS.configBytes) {
      error = `${f.name}: the config is larger than ${LIMITS.configBytes / 1024} KB.`;
      return;
    }
    const parsed = await client.call(async (api) => api.parseConfig(await f.text()));
    if (parsed.status === 'error') {
      error = `${f.name}: ${parsed.error.message}`;
      return;
    }
    const sections = parsed.value.sections;
    if (sections.length === 0) {
      error = `${f.name}: the config has no palettes.`;
    } else if (sections.length === 1) {
      await importSection(sections[0]!, f.name);
    } else {
      configSections = sections;
      configName = f.name;
      sizeDialog.showModal();
    }
  }

  async function importSection(section: ConfigSection, name: string) {
    sizeDialog?.close();
    const gen = generation;
    // Svelte state is a proxy, which can't be posted to the worker: send a plain copy.
    const plain = $state.snapshot(section);
    const resolved = await client.call((api) => api.resolveSection(plain));
    if (gen !== generation) return;
    if (resolved.status === 'error') {
      error = `${name}: ${resolved.error.message}`;
      return;
    }
    // Every pick resolved: replace the list in one step, keeping the config's order.
    picks = resolved.value.map((p) => ({
      id: nextPickId++,
      pixel: p.pixel,
      matching: p.matching,
      ink: p.ink,
      alternatives: p.alternatives,
    }));
    status = `Imported ${picks.length} pick${picks.length === 1 ? '' : 's'} (size ${section.size}) from ${name}.`;
    requestRecolor();
  }

  async function exportPicks() {
    if (!file) return;
    const exported = await client.call((api) =>
      api.exportConfig(
        file!.name,
        picks.map((p) => ({ rgba: $state.snapshot(p.pixel), ink: p.ink.name })),
      ),
    );
    if (exported.status === 'error') {
      error = exported.error.message;
      return;
    }
    const url = URL.createObjectURL(new Blob([exported.value], { type: 'application/toml' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `${file.name.replace(/\.[^.]+$/, '')}.palettes.toml`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 10_000);
  }

  // ── Live recolor (W10 vi a): one in flight, one pending; stale results dropped ─────────────────
  interface RecolorRequest {
    generation: number;
    revision: number;
    mappings: ReturnType<typeof mappings>;
  }

  const recolorer = new Latest<RecolorRequest>(async (req) => {
    recoloring = true;
    const outcome = await client.call((api) =>
      api.recolor(req.generation, req.revision, req.mappings),
    );
    recoloring = recolorer.busy && !(req.generation === generation && req.revision === revision);
    if (outcome.status === 'error') {
      if (outcome.error.kind !== 'superseded' && req.generation === generation) {
        error = outcome.error.message;
      }
      return;
    }
    const { bitmap } = outcome.value;
    if (req.generation === generation && req.revision === revision) {
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
    recolorer.request({ generation, revision, mappings: $state.snapshot(mappings(picks)) });
  }

  // ── Download (W9 a): always the current revision at full resolution ───────────────────────────
  async function download() {
    if (!file) return;
    await recolorer.idle();
    const gen = generation;
    const rev = revision;
    if (resultRevision !== rev) return;
    const png: AppOutcome<Blob> = await client.call((api) => api.encodePng(gen, rev));
    if (png.status === 'error') {
      if (png.error.kind !== 'superseded') error = png.error.message;
      return;
    }
    const url = URL.createObjectURL(png.value);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${file.name.replace(/\.[^.]+$/, '')}-rekolor.png`;
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
  <section class="toolbar" aria-label="Image and zoom">
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
        {#if stats}· {stats.colors.toLocaleString()} colors{:else}· counting colors…{/if}
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
    <ImageView
      label="Original — click to pick a color"
      bitmap={original}
      {view}
      markers={picks.flatMap((p) => (p.at ? [p.at] : []))}
      placeholder={opening
        ? 'Opening…'
        : 'Open, drop or paste an image (PNG, JPEG, WebP, GIF, AVIF…)'}
      onview={(v) => (view = v)}
      onviewport={setViewport}
      onpick={pickAt}
    />
    <ImageView
      label={!image
        ? 'Preview'
        : picks.length === 0
          ? 'No picks yet: the image as printed on white'
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
        <button type="button" onclick={() => importSection(section, configName)}>
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
    <PickList {picks} onink={changeInk} onremove={removePick} />
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
