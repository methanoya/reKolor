// Runs async tasks one after another, in the order they were queued (review fix): pick-list
// changes (pick, ink change, remove, clear, import) apply in the order of the user's actions, even
// when an earlier one waits for the worker.

export class Serial {
  #tail: Promise<unknown> = Promise.resolve();

  run<T>(task: () => T | Promise<T>): Promise<T> {
    const result = this.#tail.then(task);
    this.#tail = result.catch(() => {});
    return result;
  }
}
