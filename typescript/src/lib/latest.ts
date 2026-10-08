// "Latest only" scheduling for live recolor (W10 vi a): at most one run in flight and one pending.
// A new request while a run is in flight replaces the pending one, so rapid pick changes cost at
// most one extra run, not one per change. Runs can't be interrupted (Rust recolor is synchronous).

export class Latest<P> {
  #run: (request: P) => Promise<void>;
  #inFlight: Promise<void> | undefined;
  #pending: { request: P } | undefined;
  #idle: (() => void)[] = [];

  constructor(run: (request: P) => Promise<void>) {
    this.#run = run;
  }

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
    return this.#inFlight ? new Promise((resolve) => this.#idle.push(resolve)) : Promise.resolve();
  }

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
