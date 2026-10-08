<script lang="ts">
  // One canvas view (original or result). Draws the bitmap through the shared zoom/pan view; drag
  // pans, the wheel zooms, a click (without dragging) picks the source pixel under the pointer, and
  // a Shift-drag that starts on a marker moves that pick.

  import type { Rgba } from 'rekolor-wasm';
  import type { PickMove } from '../lib/picks';
  import {
    clampedPixel,
    imagePixel,
    markerAt,
    pan,
    pixelCenter,
    zoomAt,
    type Size,
    type View,
  } from '../lib/view';

  interface Props {
    label: string;
    bitmap: ImageBitmap | undefined;
    view: View;
    /** Source pixels to mark (picks made by clicking), by pick id. */
    markers?: { id: number; x: number; y: number }[];
    /** Source pixels to mark with a square (colors left unprinted, U3); not movable. */
    squares?: { x: number; y: number }[];
    placeholder: string;
    onview: (view: View) => void;
    onviewport: (size: Size) => void;
    onpick?: (x: number, y: number, seen: Rgba | undefined) => void;
    onmove?: (move: PickMove) => void;
    /** Escape, or the pointer was taken away (`pointercancel`), during a pick move. */
    onmovecancel?: (drag: number) => void;
    /**
     * A CSS color behind the image instead of the checkerboard (the material, M5 a). Drawn as the
     * frame's background, never into the image canvas, which must hold image pixels only (C5 a).
     */
    backdrop?: string | undefined;
  }

  let {
    label,
    bitmap,
    view,
    markers = [],
    squares = [],
    placeholder,
    onview,
    onviewport,
    onpick,
    onmove,
    onmovecancel,
    backdrop,
  }: Props = $props();

  /** How far from a marker's center (CSS pixels) a Shift-drag still grabs it. */
  const GRAB_RADIUS = 10;

  let frame: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  /** The markers, drawn on their own canvas so the image canvas holds only image pixels. */
  let overlay: HTMLCanvasElement;
  let size = $state<Size>({ width: 0, height: 0 });
  let dpr = $state(1);

  $effect(() => {
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      size = { width: entry.contentRect.width, height: entry.contentRect.height };
      dpr = window.devicePixelRatio || 1;
      onviewport(size);
    });
    observer.observe(frame);
    return () => observer.disconnect();
  });

  // Redraw whenever the bitmap, view or size change.
  $effect(() => {
    const context = canvas.getContext('2d');
    if (!context) return;
    canvas.width = Math.max(1, Math.round(size.width * dpr));
    canvas.height = Math.max(1, Math.round(size.height * dpr));
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.clearRect(0, 0, canvas.width, canvas.height);
    if (!bitmap) return;
    const k = dpr * view.scale;
    context.setTransform(k, 0, 0, k, -view.x * k, -view.y * k);
    // Crisp pixels when zoomed in (and exact colors for the mismatch check); smooth when reduced.
    context.imageSmoothingEnabled = view.scale < 1;
    context.drawImage(bitmap, 0, 0);
  });

  // The markers, on top: redrawn on their own while a pick is dragged.
  $effect(() => {
    const context = overlay.getContext('2d');
    if (!context) return;
    overlay.width = Math.max(1, Math.round(size.width * dpr));
    overlay.height = Math.max(1, Math.round(size.height * dpr));
    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    if (!bitmap) return;
    const outline = () => {
      context.lineWidth = 3;
      context.strokeStyle = 'rgb(0 0 0 / 0.75)';
      context.stroke();
      context.lineWidth = 1.5;
      context.strokeStyle = 'white';
      context.stroke();
    };
    for (const m of markers) {
      const { px, py } = pixelCenter(view, m.x, m.y);
      context.beginPath();
      context.arc(px, py, 6, 0, 2 * Math.PI);
      outline();
    }
    for (const s of squares) {
      const { px, py } = pixelCenter(view, s.x, s.y);
      context.beginPath();
      context.rect(px - 5.5, py - 5.5, 11, 11);
      outline();
    }
  });

  // The wheel listener must be non-passive to stop the page from scrolling while zooming.
  $effect(() => {
    const onWheel = (e: WheelEvent) => {
      if (!bitmap) return;
      e.preventDefault();
      const { px, py } = position(e);
      onview(zoomAt(view, Math.exp(-e.deltaY * 0.0015), px, py));
    };
    canvas.addEventListener('wheel', onWheel, { passive: false });
    return () => canvas.removeEventListener('wheel', onWheel);
  });

  const position = (e: { clientX: number; clientY: number }) => {
    const rect = canvas.getBoundingClientRect();
    return { px: e.clientX - rect.left, py: e.clientY - rect.top };
  };

  let drag: { id: number; px: number; py: number; view: View; moved: boolean } | undefined;

  /** A pick being moved: its marker's id, the pointer that moves it, and the last pixel reported. */
  let moving = $state<{
    id: number;
    drag: number;
    pointerId: number;
    x: number;
    y: number;
    moved: boolean;
  }>();
  let drags = 0;
  /** Shift is held and the pointer is over a marker: it can be grabbed. */
  let grabbable = $state(false);
  let shift = false;
  let pointer: { px: number; py: number } | undefined;

  const grabTarget = (px: number, py: number) =>
    onmove && bitmap ? markerAt(view, markers, px, py, GRAB_RADIUS) : undefined;

  function updateGrabbable() {
    grabbable = shift && !!pointer && !!grabTarget(pointer.px, pointer.py);
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && e.type === 'keydown' && moving) {
      e.preventDefault();
      cancelMove();
      return;
    }
    if (e.key !== 'Shift') return;
    shift = e.type === 'keydown';
    updateGrabbable();
  }

  function onpointerdown(e: PointerEvent) {
    // One gesture at a time: a second pointer can't replace or join the one in progress (review fix).
    if (moving || drag || !bitmap || e.button !== 0) return;
    canvas.setPointerCapture(e.pointerId);
    const { px, py } = position(e);
    const grabbed = e.shiftKey ? grabTarget(px, py) : undefined;
    if (grabbed) {
      moving = {
        id: grabbed.id,
        drag: ++drags,
        pointerId: e.pointerId,
        x: grabbed.x,
        y: grabbed.y,
        moved: false,
      };
      return;
    }
    drag = { id: e.pointerId, px, py, view, moved: false };
  }

  function onpointermove(e: PointerEvent) {
    // A move belongs to the pointer that started it; other pointers can't drive it (review fix).
    if (moving && e.pointerId !== moving.pointerId) return;
    pointer = position(e);
    shift = e.shiftKey;
    if (moving && bitmap) {
      const { x, y } = clampedPixel(view, pointer.px, pointer.py, bitmap);
      if (x === moving.x && y === moving.y) return;
      moving = { ...moving, x, y, moved: true };
      onmove?.({ drag: moving.drag, id: moving.id, x, y, seen: seenColor(x, y), done: false });
      return;
    }
    if (!drag) updateGrabbable();
    if (!drag || e.pointerId !== drag.id) return;
    const { px, py } = pointer;
    const dx = px - drag.px;
    const dy = py - drag.py;
    if (!drag.moved && Math.hypot(dx, dy) < 4) return;
    drag.moved = true;
    onview(pan(drag.view, dx, dy));
  }

  /** Ends a pick move on release: the last pixel is final. */
  function endMove() {
    if (!moving) return;
    const { drag: d, id, x, y, moved } = moving;
    moving = undefined;
    if (moved) onmove?.({ drag: d, id, x, y, seen: seenColor(x, y), done: true });
  }

  /** Cancels a pick move (F1 a): the app puts the pick back as it was. */
  function cancelMove() {
    if (!moving) return;
    const { drag: d, pointerId } = moving;
    moving = undefined;
    if (canvas.hasPointerCapture(pointerId)) canvas.releasePointerCapture(pointerId);
    onmovecancel?.(d);
  }

  /**
   * The browser took the pointer away (`pointercancel`), or its capture was lost without a release:
   * a move is cancelled (F1 a), a pan is dropped. After a release or a cancel nothing is active, so
   * the `lostpointercapture` that follows them does nothing.
   */
  function endGesture(e: PointerEvent) {
    if (moving?.pointerId === e.pointerId) cancelMove();
    else if (drag?.id === e.pointerId) drag = undefined;
  }

  function onpointerup(e: PointerEvent) {
    if (moving) {
      if (e.pointerId === moving.pointerId) endMove();
      return;
    }
    if (!drag || e.pointerId !== drag.id) return;
    const wasClick = !drag.moved;
    drag = undefined;
    if (!wasClick || !onpick || !bitmap) return;
    const { px, py } = position(e);
    const pixel = imagePixel(view, px, py, bitmap);
    if (pixel) onpick(pixel.x, pixel.y, seenColor(pixel.x, pixel.y));
  }

  /**
   * The color shown at a source pixel, for Rust's mismatch warning (R10). Only at 100 % and above,
   * where pixels are drawn unsmoothed; reduced views blend neighbors, so nothing is sent then.
   */
  function seenColor(x: number, y: number): Rgba | undefined {
    if (view.scale < 1) return undefined;
    const { px, py } = pixelCenter(view, x, y);
    // A pick dragged past the edge can sit on a pixel scrolled out of sight.
    if (px < 0 || py < 0 || px >= size.width || py >= size.height) return undefined;
    const data = canvas
      .getContext('2d')
      ?.getImageData(Math.floor(px * dpr), Math.floor(py * dpr), 1, 1).data;
    if (!data) return undefined;
    const [r = 0, g = 0, b = 0, a = 0] = data;
    return { r, g, b, a };
  }
</script>

<svelte:window onkeydown={onkey} onkeyup={onkey} />

<figure class="view">
  <figcaption>{label}</figcaption>
  <div class="frame" class:empty={!bitmap} style:background={backdrop} bind:this={frame}>
    <canvas
      bind:this={canvas}
      class:pickable={!!onpick && !!bitmap}
      class:grabbable
      class:moving={!!moving}
      aria-label={label}
      {onpointerdown}
      {onpointermove}
      {onpointerup}
      onpointercancel={endGesture}
      onlostpointercapture={endGesture}
      onpointerleave={() => {
        pointer = undefined;
        grabbable = false;
      }}
    ></canvas>
    <canvas bind:this={overlay} class="overlay" aria-hidden="true"></canvas>
    {#if !bitmap && placeholder}
      <p class="placeholder">{placeholder}</p>
    {/if}
  </div>
</figure>

<style>
  .view {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
  }
  figcaption {
    font-size: 0.9rem;
    color: var(--muted);
  }
  .frame {
    position: relative;
    height: min(62vh, 640px);
    min-height: 240px;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background:
      repeating-conic-gradient(var(--checker) 0% 25%, transparent 0% 50%) 50% / 16px 16px,
      var(--surface);
  }
  @media (max-width: 700px) {
    .frame {
      height: 45vh;
    }
  }
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    touch-action: none;
    cursor: grab;
  }
  canvas.pickable {
    cursor: crosshair;
  }
  canvas.grabbable {
    cursor: grab;
  }
  canvas.moving {
    cursor: grabbing;
  }
  .overlay {
    pointer-events: none;
  }
  .empty canvas {
    cursor: default;
  }
  .placeholder {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    padding: 1rem;
    text-align: center;
    color: var(--muted);
  }
</style>
