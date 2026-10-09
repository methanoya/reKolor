// Runs async tasks one after another, in the order they were queued: pick-list changes (pick, ink
// change, remove, clear, import, material) apply in the order of the user's actions, even when an
// earlier one waits for the worker.

// How it works: the queue is a chain of promises. Each new task is attached with `.then` to the end
// of the chain (`#tail`), so it starts only after the previous one finishes. A `Promise` is
// JavaScript's handle for a result that arrives later. Names starting with `#` are private fields
// of the class.
export class Serial {
  #tail: Promise<unknown> = Promise.resolve();
  /** Tasks queued so far: tells a coalescer whether its task is still the last one. */
  #queued = 0;

  run<T>(task: () => T | Promise<T>): Promise<T> {
    this.#queued++;
    const result = this.#tail.then(task);
    // The chain continues even if this task fails; the caller still sees the failure through
    // `result`.
    this.#tail = result.catch(() => {});
    return result;
  }

  // Returns a function. Each call checks whether the task it queued last time is still waiting
  // at the end of the queue (its `position` equals the current count); if so, it only swaps in the
  // new value. Used for the material color picker and the ΔE sliders, which fire many changes
  // while dragging.
  coalescing<V>(apply: (value: V) => void | Promise<void>): (value: V) => void {
    let waiting: { value: V; position: number } | undefined;
    return (value) => {
      if (waiting?.position === this.#queued) {
        waiting.value = value;
        return;
      }
      const slot = { value, position: 0 };
      waiting = slot;
      // `void` marks the returned promise as deliberately not awaited.
      void this.run(() => {
        if (waiting === slot) waiting = undefined;
        return apply(slot.value);
      });
      slot.position = this.#queued;
    };
  }
}
