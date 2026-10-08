<script lang="ts">
  import type { Rgb } from 'rekolor-wasm';
  import { WHITE, css, cssAlpha, hex, sameRgb, type PickEntry } from '../lib/picks';

  interface Props {
    picks: PickEntry[];
    /** The material the picks are matched on. */
    material: Rgb;
    onink: (id: number, index: number) => void;
    onremove: (id: number) => void;
  }

  let { picks, material, onink, onremove }: Props = $props();

  const on = $derived(sameRgb(material, WHITE) ? 'On white' : 'On the material');
</script>

{#if picks.length === 0}
  <p class="hint">
    Click a color in the original to pick it. Each pick gets the nearest ink; change it from the
    list.
  </p>
{:else}
  <ol class="picks" aria-label="Picked colors">
    {#each picks as pick (pick.id)}
      {@const label = `rgb ${pick.pixel.r}, ${pick.pixel.g}, ${pick.pixel.b}${pick.pixel.a < 255 ? `, alpha ${pick.pixel.a}` : ''}`}
      <li class="pick">
        <div class="swatches">
          <span class="swatch checker" title="Stored pixel: {label}">
            <span style:background={cssAlpha(pick.pixel)}></span>
          </span>
          <span
            class="swatch"
            style:background={css(pick.matching)}
            title="{on}: {hex(pick.matching)}"
          ></span>
          <span class="arrow" aria-hidden="true">→</span>
          <span class="swatch ink" style:background={css(pick.ink.rgb)} title={pick.ink.name}
          ></span>
        </div>
        <div class="details">
          <span class="color">{label}</span>
          <label class="ink-choice">
            <span class="visually-hidden">Ink for {label}</span>
            <select
              value={pick.alternatives.findIndex((a) => a.index === pick.ink.index)}
              onchange={(e) => onink(pick.id, Number(e.currentTarget.value))}
            >
              <!-- Customizable select: swatches where supported, plain text elsewhere (Firefox). -->
              <button><selectedcontent></selectedcontent></button>
              {#each pick.alternatives as alt, i (alt.index)}
                <option value={i}
                  ><span class="option-swatch" style:background={css(alt.rgb)}></span><span
                    class="option-text">{`${alt.name} · ΔE ${alt.deltaE.toFixed(1)}`}</span
                  ></option
                >
              {/each}
            </select>
          </label>
          {#if pick.mismatch}
            <span class="warning" role="note">
              Shown as {hex(pick.mismatch.seen)}, stored as {hex(pick.mismatch.stored)}
            </span>
          {/if}
        </div>
        <button
          type="button"
          class="remove"
          aria-label="Remove pick {label}"
          title="Remove"
          onclick={() => onremove(pick.id)}>×</button
        >
      </li>
    {/each}
  </ol>
{/if}

<style>
  .hint {
    color: var(--muted);
  }
  .picks {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 380px), 1fr));
    gap: 0.75rem;
  }
  .pick {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.6rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
  }
  .swatches {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .swatch {
    width: 1.6rem;
    height: 2.4rem;
    border-radius: 4px;
    border: 1px solid var(--border);
    display: block;
  }
  .swatch.ink {
    width: 2.4rem;
  }
  .checker {
    background: repeating-conic-gradient(#ccc 0% 25%, #fff 0% 50%) 50% / 8px 8px;
    overflow: hidden;
  }
  .checker > span {
    display: block;
    width: 100%;
    height: 100%;
  }
  .arrow {
    color: var(--muted);
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }
  .color {
    font-size: 0.85rem;
    color: var(--muted);
  }
  select {
    width: 100%;
    font: inherit;
    padding: 0.25rem;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--fg);
  }
  @supports (appearance: base-select) and selector(::picker(select)) {
    select,
    ::picker(select) {
      appearance: base-select;
    }
    ::picker(select) {
      border: 1px solid var(--border);
      border-radius: 6px;
      background: var(--surface);
      color: var(--fg);
    }
    option {
      display: flex;
      align-items: center;
      gap: 0.4rem;
      padding: 0.3rem 0.5rem;
    }
    option:checked {
      font-weight: 600;
    }
    .option-swatch {
      flex: none;
      width: 1.1rem;
      height: 1.1rem;
      border-radius: 3px;
      border: 1px solid var(--border);
    }
    selectedcontent {
      display: flex;
      align-items: center;
      gap: 0.4rem;
      min-width: 0;
    }
    /* A name too long for the closed select ends in "…" instead of a cut-off glyph. */
    selectedcontent .option-text {
      min-width: 0;
      overflow: hidden;
      white-space: nowrap;
      text-overflow: ellipsis;
    }
  }
  .warning {
    font-size: 0.8rem;
    color: var(--warning);
  }
  .remove {
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
  .remove:hover {
    border-color: var(--border);
    color: var(--fg);
  }
</style>
