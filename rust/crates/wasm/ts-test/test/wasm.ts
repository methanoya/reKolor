// Loads the `rekolor-wasm` package (built with `wasm-pack --target web`) the way an app would,
// except that the `.wasm` bytes come from disk instead of `fetch`.

import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import init, { type Outcome } from 'rekolor-wasm';

let ready: Promise<unknown> | undefined;

export function loadWasm(): Promise<unknown> {
  ready ??= readFile(
    createRequire(import.meta.url).resolve('rekolor-wasm/rekolor_wasm_bg.wasm'),
  ).then((bytes) => init({ module_or_path: bytes }));
  return ready;
}

/** The value of an `ok` outcome; fails the test with the error otherwise. */
export function unwrap<T>(outcome: Outcome<T>): T {
  if (outcome.status === 'error') {
    throw new Error(`expected ok, got ${outcome.error.kind}: ${outcome.error.message}`);
  }
  return outcome.value;
}
