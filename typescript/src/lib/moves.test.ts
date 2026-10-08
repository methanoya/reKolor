// The pick-move rules (`.agents/repositioning`) with controlled promises: coalescing, the final
// update, a second drag before the first settles, a reset, and cancel (F1 a).

import { describe, expect, test } from 'vitest';
import { Mover, type Marker } from './moves';
import type { PickMove } from './picks';
import { Serial } from './serial';

/** A promise the test resolves when it wants. */
function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => (resolve = r));
  return { promise, resolve };
}

const settle = () => new Promise((r) => setTimeout(r, 0));

const move = (drag: number, x: number, done = false): PickMove => ({
  drag,
  id: 1,
  x,
  y: 0,
  seen: undefined,
  done,
});

/** One pick whose `x` the fake apply sets; each apply waits for its gate when one is set. */
function setup() {
  const queue = new Serial();
  const pick = { id: 1, x: 0 };
  const applied: number[] = [];
  const restored: ({ id: number; x: number } | undefined)[] = [];
  const gates: ReturnType<typeof deferred>[] = [];
  let marker: Marker | undefined;
  const mover = new Mover<{ id: number; x: number }>({
    queue: (task) => queue.run(task),
    apply: async (m) => {
      const gate = gates.shift();
      if (gate) await gate.promise;
      applied.push(m.x);
      pick.x = m.x;
    },
    save: () => ({ ...pick }),
    restore: (saved) => {
      restored.push(saved);
      if (saved) pick.x = saved.x;
    },
    show: (m) => (marker = m),
  });
  /** Holds the next apply until the returned gate is resolved. */
  const hold = () => {
    const gate = deferred();
    gates.push(gate);
    return gate;
  };
  return { mover, queue, pick, applied, restored, hold, marker: () => marker };
}

describe('Mover', () => {
  test('updates during a running apply coalesce to the newest one', async () => {
    const { mover, applied, hold, marker } = setup();
    const gate = hold();
    mover.update(move(1, 10));
    await settle();
    for (const x of [11, 12, 13]) mover.update(move(1, x));
    expect(marker()).toEqual({ drag: 1, id: 1, x: 13, y: 0 }); // the marker follows at once
    gate.resolve();
    await settle();
    expect(applied).toEqual([10, 13]);
  });

  test('the release is applied, and then the marker is drawn at the pick again', async () => {
    const { mover, applied, hold, marker } = setup();
    const gate = hold();
    mover.update(move(1, 10));
    await settle();
    mover.update(move(1, 11));
    mover.update(move(1, 12, true));
    expect(marker()?.x).toBe(12);
    gate.resolve();
    await settle();
    expect(applied).toEqual([10, 12]);
    expect(marker()).toBeUndefined();
  });

  test('a second drag before the first settles: both finals apply, in order', async () => {
    const { mover, applied, hold, marker } = setup();
    const gate = hold();
    mover.update(move(1, 10));
    await settle();
    mover.update(move(1, 12, true)); // drag 1's release, queued behind the running apply
    mover.update(move(2, 20)); // drag 2 starts: a new slot, not merged into drag 1's release
    mover.update(move(2, 21, true));
    gate.resolve();
    await settle();
    expect(applied).toEqual([10, 12, 21]);
    expect(marker()).toBeUndefined();
  });

  test("drag 1 settling does not clear drag 2's marker", async () => {
    const { mover, hold, marker } = setup();
    const gate = hold();
    mover.update(move(1, 10, true));
    await settle();
    mover.update(move(2, 20));
    gate.resolve();
    await settle();
    expect(marker()).toEqual({ drag: 2, id: 1, x: 20, y: 0 });
  });

  test('a reset drops the queued updates and the marker', async () => {
    const { mover, applied, hold, marker } = setup();
    const gate = hold();
    mover.update(move(1, 10));
    await settle();
    mover.update(move(1, 11, true));
    mover.reset();
    expect(marker()).toBeUndefined();
    gate.resolve();
    await settle();
    expect(applied).toEqual([10]); // the running one is the app's to drop (generation check)
  });

  test('a cancel puts the pick back as it was before the drag', async () => {
    const { mover, pick, applied, restored, hold, marker } = setup();
    mover.update(move(1, 10));
    await settle();
    const gate = hold();
    mover.update(move(1, 11));
    await settle();
    mover.update(move(1, 12)); // queued behind the running apply: dropped by the cancel
    mover.cancel(1);
    gate.resolve();
    await settle();
    expect(applied).toEqual([10, 11]);
    expect(restored).toEqual([{ id: 1, x: 0 }]);
    expect(pick.x).toBe(0);
    expect(marker()).toBeUndefined();
  });

  test('a cancel before any update applied restores nothing', async () => {
    const { mover, queue, applied, restored, marker } = setup();
    const busy = deferred();
    void queue.run(() => busy.promise); // an earlier pick-list change still running
    mover.update(move(1, 10));
    mover.cancel(1);
    busy.resolve();
    await settle();
    expect(applied).toEqual([]);
    expect(restored).toEqual([undefined]);
    expect(marker()).toBeUndefined();
  });

  test('each drag saves its own starting point', async () => {
    const { mover, restored } = setup();
    mover.update(move(1, 10, true));
    await settle();
    mover.update(move(2, 20));
    await settle();
    mover.cancel(2);
    await settle();
    expect(restored).toEqual([{ id: 1, x: 10 }]); // where drag 1 left it, not the original 0
  });

  test('a cancel while the release is being applied still restores (review fix)', async () => {
    const { mover, pick, applied, restored, hold, marker } = setup();
    mover.update(move(1, 10));
    await settle();
    const gate = hold();
    mover.update(move(1, 12, true)); // the release, now inside apply
    await settle();
    mover.cancel(1);
    gate.resolve();
    await settle();
    expect(applied).toEqual([10, 12]);
    expect(restored).toEqual([{ id: 1, x: 0 }]);
    expect(pick.x).toBe(0);
    expect(marker()).toBeUndefined();
  });
});
