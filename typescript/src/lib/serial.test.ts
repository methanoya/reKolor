import { describe, expect, test } from 'vitest';
import { Serial } from './serial';

describe('Serial', () => {
  test('a slow earlier task finishes before a later fast one starts', async () => {
    const serial = new Serial();
    const log: string[] = [];
    let release!: () => void;
    const slow = serial.run(async () => {
      await new Promise<void>((r) => (release = r));
      log.push('pick');
    });
    const fast = serial.run(() => log.push('clear'));
    await new Promise((r) => setTimeout(r, 0));
    expect(log).toEqual([]);
    release();
    await Promise.all([slow, fast]);
    expect(log).toEqual(['pick', 'clear']);
  });

  test('a failing task does not block the next one', async () => {
    const serial = new Serial();
    await expect(serial.run(() => Promise.reject(new Error('boom')))).rejects.toThrow('boom');
    await expect(serial.run(() => 42)).resolves.toBe(42);
  });
});
