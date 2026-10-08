// The app's result type. Same shape as the WASM package's `Outcome`, with the app's own error
// kinds added; it crosses the worker boundary as plain data (thrown errors would lose their kind).

import type { ErrorKind } from 'rekolor-wasm';

export type AppErrorKind =
  | ErrorKind
  /** The engine hasn't finished starting (WASM or palette not loaded yet). */
  | 'notReady'
  /** The engine couldn't start. */
  | 'initFailed'
  /** The worker crashed; it has been restarted. */
  | 'workerFailed'
  /** The browser couldn't decode the file. */
  | 'decodeFailed'
  /** The file or image exceeds the app's limits. */
  | 'fileTooLarge'
  | 'imageTooLarge'
  /** A newer image or revision replaced the one this call was for. */
  | 'superseded'
  /** No image is open. */
  | 'noImage'
  /** A palette config couldn't be read or doesn't fit the palette. */
  | 'invalidConfig';

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

export type AppOutcome<T> = { status: 'ok'; value: T } | { status: 'error'; error: AppError };

export const ok = <T>(value: T): AppOutcome<T> => ({ status: 'ok', value });
export const err = <T = never>(kind: AppErrorKind, message: string): AppOutcome<T> => ({
  status: 'error',
  error: { kind, message },
});
