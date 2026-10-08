// Runs async tasks one after another, in the order they were queued: pick-list
// changes (pick, ink change, remove, clear, import, material) apply in the order of the user's
// actions, even when an earlier one waits for the worker.

export class Serial {
  #tail: Promise<unknown> = Promise.resolve();
  /** Tasks queued so far: tells a coalescer whether its task is still the last one. */
  #queued = 0;

  run<T>(task: () => T | Promise<T>): Promise<T> {
    this.#queued++;
    const result = this.#tail.then(task);
    this.#tail = result.catch(() => {});
    return result;
  }

  /**
   * A coalescing entry point: `push(value)` queues `apply(value)`, unless
   * this coalescer's previous task hasn't started and is still the last task queued; then that
   * task takes the newer value instead. A burst of pushes is one task, and a push never moves
   * ahead of anything queued before it.
   */
  coalescing<V>(apply: (value: V) => void | Promise<void>): (value: V) => void {
    let waiting: { value: V; position: number } | undefined;
    return (value) => {
      if (waiting?.position === this.#queued) {
        waiting.value = value;
        return;
      }
      const slot = { value, position: 0 };
      waiting = slot;
      void this.run(() => {
        if (waiting === slot) waiting = undefined;
        return apply(slot.value);
      });
      slot.position = this.#queued;
    };
  }
}
