// "Latest only" scheduling for live recolor: at most one run in flight and one pending.
// A new request while a run is in flight replaces the pending one, so rapid pick changes cost at
// most one extra run, not one per change. Runs can't be interrupted (Rust recolor is synchronous).

// `<P>` is the request type (a type parameter), chosen by the code that creates the scheduler.
// `{ request: P } | undefined` wraps the pending request in an object, so "nothing pending" can't
// be confused with a request that happens to be `undefined`.
export class Latest<P> {
  #run: (request: P) => Promise<void>;
  #inFlight: Promise<void> | undefined;
  #pending: { request: P } | undefined;
  #idle: (() => void)[] = [];

  constructor(run: (request: P) => Promise<void>) {
    this.#run = run;
  }

  // `get` defines a property computed on read: `latest.busy`, not `latest.busy()`.
  get busy(): boolean {
    return this.#inFlight !== undefined;
  }

  request(request: P): void {
    if (this.#inFlight) {
      this.#pending = { request };
    } else {
      this.#start(request);
    }
  }

  /**
   * Drops the pending request (the running one can't be stopped; its result is ignored by the
   * caller's generation check). Used when the worker restarts.
   */
  cancel(): void {
    this.#pending = undefined;
  }

  /** Resolves when nothing is running or pending. */
  idle(): Promise<void> {
    // If busy, return a promise whose `resolve` function is stored, to be called when the queue
    // drains.
    return this.#inFlight ? new Promise((resolve) => this.#idle.push(resolve)) : Promise.resolve();
  }

  // Runs the request; when it settles (success or failure), starts the pending one if any, or else
  // wakes everyone waiting in `idle()`. `splice(0)` empties the array and returns its contents.
  #start(request: P): void {
    this.#inFlight = this.#run(request)
      .catch(() => {})
      .then(() => {
        const next = this.#pending;
        this.#pending = undefined;
        this.#inFlight = undefined;
        if (next) {
          this.#start(next.request);
        } else {
          for (const resolve of this.#idle.splice(0)) resolve();
        }
      });
  }
}
