// Unit tests for the build lock in `wasm-watch.ts`, on a lock file in a temporary folder.

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { tryLock, unlock } from './wasm-watch.ts';

let dir: string;
let file: string;
beforeEach(() => {
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rekolor-lock-'));
  file = path.join(dir, 'target', 'build.lock');
});
afterEach(() => fs.rmSync(dir, { recursive: true, force: true }));

/** Writes a lock as another build would have, `minutesAgo` old. */
function lockAs(content: string, minutesAgo = 0) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, content);
  const time = new Date(Date.now() - minutesAgo * 60_000);
  fs.utimesSync(file, time, time);
}

describe('the WASM build lock', () => {
  test('one holder at a time, and only the holder releases it', () => {
    const token = tryLock(file);
    expect(token).toBeDefined();
    expect(tryLock(file)).toBeUndefined();
    unlock(file, 'another build');
    expect(fs.existsSync(file)).toBe(true);
    unlock(file, token!);
    expect(fs.existsSync(file)).toBe(false);
    expect(tryLock(file)).toBeDefined();
  });

  test('a build running for minutes keeps its lock', () => {
    lockAs(`${process.pid} running`, 5);
    expect(tryLock(file)).toBeUndefined();
    expect(fs.readFileSync(file, 'utf8')).toBe(`${process.pid} running`);
  });

  test("a killed owner's lock is broken, and its late unlock can't remove the new one", () => {
    // A process that has already exited: its ID no longer runs.
    const { pid } = spawnSync(process.execPath, ['-e', '']);
    lockAs(`${pid} killed`);
    expect(tryLock(file)).toBeUndefined(); // breaks it
    const token = tryLock(file);
    expect(token).toBeDefined();
    unlock(file, `${pid} killed`);
    expect(fs.readFileSync(file, 'utf8')).toBe(token);
  });

  test('a lock older than 30 minutes is broken even if its ID runs (it may be reused)', () => {
    lockAs(`${process.pid} ancient`, 31);
    expect(tryLock(file)).toBeUndefined();
    expect(tryLock(file)).toBeDefined();
  });

  test('a lock without a readable owner yet is kept while fresh', () => {
    lockAs('');
    expect(tryLock(file)).toBeUndefined();
    expect(fs.existsSync(file)).toBe(true);
  });
});
