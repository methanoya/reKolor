<script lang="ts">
  // One canvas view (original or result). Draws the bitmap through the shared zoom/pan view; drag
  // pans, the wheel zooms, a click (without dragging) picks the source pixel under the pointer.

  import type { Rgba } from 'rekolor-wasm';
  import { imagePixel, pan, pixelCenter, zoomAt, type Size, type View } from '../lib/view';

  interface Props {
    label: string;
    bitmap: ImageBitmap | undefined;
    view: View;
    /** Source pixels to mark (picks made by clicking). */
    markers?: { x: number; y: number }[];
    placeholder: string;
    onview: (view: View) => void;
    onviewport: (size: Size) => void;
    onpick?: (x: number, y: number, seen: Rgba | undefined) => void;
  }

  let {
    label,
    bitmap,
    view,
    markers = [],
    placeholder,
    onview,
    onviewport,
    onpick,
  }: Props = $props();

  let frame: HTMLDivElement;
  let canvas: HTMLCanvasElement;
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

  // Redraw whenever the bitmap, view, markers or size change.
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
    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    for (const m of markers) {
      const { px, py } = pixelCenter(view, m.x, m.y);
      context.beginPath();
      context.arc(px, py, 6, 0, 2 * Math.PI);
      context.lineWidth = 3;
      context.strokeStyle = 'rgb(0 0 0 / 0.75)';
      context.stroke();
      context.lineWidth = 1.5;
      context.strokeStyle = 'white';
      context.stroke();
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

  function onpointerdown(e: PointerEvent) {
    if (!bitmap || e.button !== 0) return;
    canvas.setPointerCapture(e.pointerId);
    drag = { id: e.pointerId, ...position(e), view, moved: false };
  }

  function onpointermove(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const { px, py } = position(e);
    const dx = px - drag.px;
    const dy = py - drag.py;
    if (!drag.moved && Math.hypot(dx, dy) < 4) return;
    drag.moved = true;
    onview(pan(drag.view, dx, dy));
  }

  function onpointerup(e: PointerEvent) {
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
    const data = canvas
      .getContext('2d')
      ?.getImageData(Math.floor(px * dpr), Math.floor(py * dpr), 1, 1).data;
    if (!data) return undefined;
    const [r = 0, g = 0, b = 0, a = 0] = data;
    return { r, g, b, a };
  }
</script>

<figure class="view">
  <figcaption>{label}</figcaption>
  <div class="frame" class:empty={!bitmap} bind:this={frame}>
    <canvas
      bind:this={canvas}
      class:pickable={!!onpick && !!bitmap}
      aria-label={label}
      {onpointerdown}
      {onpointermove}
      {onpointerup}
      onpointercancel={() => (drag = undefined)}
    ></canvas>
    {#if !bitmap}
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
