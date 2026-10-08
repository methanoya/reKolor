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

  /** A queue held busy by a running first task, released by the test; `log` records what ran. */
  async function busy() {
    const serial = new Serial();
    const log: string[] = [];
    let release!: () => void;
    const first = serial.run(async () => {
      await new Promise<void>((r) => (release = r));
      log.push('first');
    });
    await new Promise((r) => setTimeout(r, 0)); // the first task is running and held
    const push = serial.coalescing<string>((value) => void log.push(`material ${value}`));
    const drain = async () => {
      release();
      await first;
      await serial.run(() => {});
    };
    return { serial, log, push, drain };
  }

  test('coalescing: a burst while the queue is busy is one task with the newest value', async () => {
    const { log, push, drain } = await busy();
    for (const value of ['A', 'B', 'C']) push(value);
    await drain();
    expect(log).toEqual(['first', 'material C']);
  });

  test('coalescing: a push never moves ahead of a task queued before it (GPT review 1)', async () => {
    const { serial, log, push, drain } = await busy();
    push('A');
    void serial.run(() => log.push('import'));
    push('B');
    push('C'); // adjacent to B: merges into it
    await drain();
    expect(log).toEqual(['first', 'material A', 'import', 'material C']);
  });

  test('coalescing: a push after its task started queues a new one', async () => {
    const serial = new Serial();
    const log: string[] = [];
    let release!: () => void;
    const push = serial.coalescing<string>(async (value) => {
      log.push(`start ${value}`);
      if (value === 'A') await new Promise<void>((r) => (release = r));
      log.push(`end ${value}`);
    });
    push('A');
    await new Promise((r) => setTimeout(r, 0)); // A is running and held
    push('B');
    push('C');
    release();
    await serial.run(() => {});
    expect(log).toEqual(['start A', 'end A', 'start C', 'end C']);
  });
});
