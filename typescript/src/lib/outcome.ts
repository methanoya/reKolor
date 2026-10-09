// The app's result type. Same shape as the WASM package's `Outcome`, with the app's own error
// kinds added; it crosses the worker boundary as plain data (thrown errors would lose their kind).

import type { ErrorKind } from 'rekolor-wasm';

// A union of string literal types: a value of this type must be one of the listed strings (or
// one of the WASM package's `ErrorKind` strings), and the compiler rejects anything else.
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
  | 'invalidConfig'
  /** An ink palette (`*.json`) couldn't be read or is too large. */
  | 'invalidPalette';

// An `interface` describes the shape of an object: which fields it has and their types.
export interface AppError {
  kind: AppErrorKind;
  message: string;
}

// Either a value or an error, told apart by `status`. After checking `status === 'ok'`, TypeScript
// lets the code read `value` (narrowing); reading it without the check is a compile error.
export type AppOutcome<T> = { status: 'ok'; value: T } | { status: 'error'; error: AppError };

// Shorthand constructors. `<T>` makes them generic; in `err`, `T = never` lets an error stand in
// for an outcome of any value type. `({ ... })` in an arrow function returns an object literal.
export const ok = <T>(value: T): AppOutcome<T> => ({ status: 'ok', value });
export const err = <T = never>(kind: AppErrorKind, message: string): AppOutcome<T> => ({
  status: 'error',
  error: { kind, message },
});
