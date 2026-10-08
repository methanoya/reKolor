// Node-only test helper: initializes the WASM package from disk (no fetch) and reads the palette.

import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import init from 'rekolor-wasm';

let ready: Promise<unknown> | undefined;

export function loadWasm(): Promise<unknown> {
  ready ??= readFile(
    createRequire(import.meta.url).resolve('rekolor-wasm/rekolor_wasm_bg.wasm'),
  ).then((bytes) => init({ module_or_path: bytes }));
  return ready;
}

export const pantoneJson = (): Promise<string> =>
  readFile(new URL('../../../palettes/pantone.json', import.meta.url), 'utf8');
