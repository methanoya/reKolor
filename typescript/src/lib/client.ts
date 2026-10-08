// Main-thread access to the worker. Every call returns an `AppOutcome`; if the worker crashes (an
// uncaught error, a message that can't be read, a WASM panic surfacing as a rejected call), the
// pending calls settle with `workerFailed`, a fresh worker is started and `onRestart` is told, so
// the user can open an image again.

import * as Comlink from 'comlink';
import { err, type AppOutcome } from './outcome';
import type { WorkerApi } from './worker';

type Api = Comlink.Remote<WorkerApi>;

export class EngineClient {
  #worker!: Worker;
  #api!: Api;
  #crashed!: Promise<never>;
  #disposed = false;
  readonly #onRestart: (reason: string) => void;

  constructor(onRestart: (reason: string) => void) {
    this.#onRestart = onRestart;
    this.#start();
  }

  #start(): void {
    this.#worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
    this.#api = Comlink.wrap<WorkerApi>(this.#worker);
    const worker = this.#worker;
    this.#crashed = new Promise<never>((_, reject) => {
      worker.addEventListener('error', (e) => reject(new Error(e.message || 'worker error')));
      worker.addEventListener('messageerror', () => reject(new Error('unreadable worker message')));
    });
    // Unobserved until a call races against it.
    this.#crashed.catch(() => {});
  }

  #restart(reason: string): void {
    if (this.#disposed) return;
    this.#api[Comlink.releaseProxy]();
    this.#worker.terminate();
    this.#start();
    this.#onRestart(reason);
  }

  /** Runs a worker call; a crash becomes a `workerFailed` outcome (and restarts the worker). */
  async call<T>(f: (api: Api) => Promise<AppOutcome<T>>): Promise<AppOutcome<T>> {
    const api = this.#api;
    try {
      return await Promise.race([f(api), this.#crashed]);
    } catch (e) {
      // An argument the browser can't send (e.g. a Svelte state proxy) is a caller bug, not a
      // crash: the worker is fine.
      if (e instanceof DOMException && e.name === 'DataCloneError') {
        return err('invalidInput', `couldn't send the request to the engine (${e.message})`);
      }
      if (api === this.#api) this.#restart(String(e));
      return err('workerFailed', `the engine stopped (${String(e)}) and was restarted`);
    }
  }

  dispose(): void {
    this.#disposed = true;
    this.#api[Comlink.releaseProxy]();
    this.#worker.terminate();
  }
}
