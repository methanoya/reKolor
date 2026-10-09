// Moving a pick by Shift-dragging its marker: the order and coalescing of the live updates, the
// cancel, and the marker drawn while a drag is in progress. The pick-list work itself is injected,
// so the rules are testable in Node with controlled promises.

import type { PickMove } from './picks';

/** Where a dragged pick's marker is drawn until its drag settles. */
export interface Marker {
  drag: number;
  id: number;
  x: number;
  y: number;
}

// The functions the app passes in (dependency injection). `Saved` is a type parameter: whatever
// the app stores to undo a drag; this file never looks inside it.
export interface MoveHooks<Saved> {
  /** Runs a task after every earlier pick-list change (the app's `Serial` queue). */
  queue: (task: () => Promise<void>) => Promise<unknown>;
  /** Re-picks the pick at the update's pixel (the app checks its image generation itself). */
  apply: (move: PickMove) => Promise<void>;
  /** The pick as it is now, kept at a drag's first applied update for a cancel. */
  save: (id: number) => Saved | undefined;
  /**
   * A cancel, in queue order: puts the saved pick back, or gets `undefined` if the drag had no
   * update applied yet (or its pick was gone).
   */
  restore: (saved: Saved | undefined) => void | Promise<void>;
  /** The dragged marker to draw, or `undefined` to draw every marker where its pick is. */
  show: (marker: Marker | undefined) => void;
}

// One queued update. `epoch` records which `reset()` period it belongs to; `dropped` marks it
// cancelled before it ran.
interface Slot {
  move: PickMove;
  epoch: number;
  dropped: boolean;
}

export class Mover<Saved> {
  readonly #hooks: MoveHooks<Saved>;
  /** Advanced by `reset()`: queued work from before it is skipped. */
  #epoch = 0;
  /** The queued update not started yet; newer updates from the same drag replace its move. */
  #pending: Slot | undefined;
  #saved: { drag: number; value: Saved | undefined } | undefined;
  #marker: Marker | undefined;

  constructor(hooks: MoveHooks<Saved>) {
    this.#hooks = hooks;
  }

  /** A drag reached a new pixel (`done` on release). The marker follows at once. */
  update(move: PickMove): void {
    this.#show({ drag: move.drag, id: move.id, x: move.x, y: move.y });
    const pending = this.#pending;
    // The same drag already has an update waiting in the queue: just give it the newer position, so
    // a fast drag costs one update per queue turn rather than one per pixel. `{ ...move }` stores a
    // copy, so later changes to the caller's object can't leak in.
    if (pending?.move.drag === move.drag && !pending.move.done) {
      pending.move = { ...move };
      return;
    }
    const slot: Slot = { move: { ...move }, epoch: this.#epoch, dropped: false };
    this.#pending = slot;
    void this.#hooks.queue(async () => {
      if (this.#pending === slot) this.#pending = undefined;
      if (slot.dropped || slot.epoch !== this.#epoch) return;
      const { move } = slot;
      // The first update of a drag that actually runs saves the pick, so a cancel can restore it.
      if (this.#saved?.drag !== move.drag) {
        this.#saved = { drag: move.drag, value: this.#hooks.save(move.id) };
      }
      // `finally` runs whether `apply` succeeds or throws.
      try {
        await this.#hooks.apply(move);
      } finally {
        if (move.done) this.#finish(move.drag);
      }
    });
  }

  /**
   * Cancels a drag (Escape, or the pointer was taken away): its update not started yet is
   * dropped, and once the one running (if any) is done, the pick is put back as it was before
   * the drag's first update.
   */
  cancel(drag: number): void {
    if (this.#pending?.move.drag === drag) {
      this.#pending.dropped = true;
      this.#pending = undefined;
    }
    // Read now: a release already being applied would clear the save before this task runs.
    const saved = this.#saved?.drag === drag ? this.#saved.value : undefined;
    const epoch = this.#epoch;
    void this.#hooks.queue(async () => {
      if (epoch !== this.#epoch) return;
      try {
        await this.#hooks.restore(saved);
      } finally {
        this.#finish(drag);
      }
    });
  }

  /** Another image opened, or the engine restarted: forget every drag. */
  reset(): void {
    this.#epoch++;
    if (this.#pending) this.#pending.dropped = true;
    this.#pending = undefined;
    this.#saved = undefined;
    this.#show(undefined);
  }

  /** A drag settled: it can no longer be cancelled, and its marker is drawn at its pick. */
  #finish(drag: number) {
    if (this.#saved?.drag === drag) this.#saved = undefined;
    if (this.#marker?.drag === drag) this.#show(undefined);
  }

  #show(marker: Marker | undefined) {
    this.#marker = marker;
    this.#hooks.show(marker);
  }
}
