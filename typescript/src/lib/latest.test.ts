import { describe, expect, test } from 'vitest';
import { Latest } from './latest';

/** A run that finishes when the test says so. */
function controlled() {
  const started: number[] = [];
  const finish: (() => void)[] = [];
  const latest = new Latest<number>(
    (n) =>
      new Promise<void>((resolve) => {
        started.push(n);
        finish.push(resolve);
      }),
  );
  const finishNext = async () => {
    finish.shift()?.();
    await new Promise((r) => setTimeout(r, 0));
  };
  return { latest, started, finishNext };
}

describe('Latest', () => {
  test('rapid requests run at most one in flight and one pending (the newest)', async () => {
    const { latest, started, finishNext } = controlled();
    for (let n = 1; n <= 10; n++) latest.request(n);
    expect(started).toEqual([1]);
    await finishNext();
    expect(started).toEqual([1, 10]);
    await finishNext();
    expect(started).toEqual([1, 10]);
    expect(latest.busy).toBe(false);
  });

  test('idle waits for the pending run too', async () => {
    const { latest, started, finishNext } = controlled();
    latest.request(1);
    latest.request(2);
    let idle = false;
    void latest.idle().then(() => (idle = true));
    await finishNext();
    expect(idle).toBe(false);
    await finishNext();
    expect(idle).toBe(true);
    expect(started).toEqual([1, 2]);
  });

  test('a failing run does not stop the next one', async () => {
    const started: number[] = [];
    const latest = new Latest<number>(async (n) => {
      started.push(n);
      if (n === 1) throw new Error('boom');
    });
    latest.request(1);
    latest.request(2);
    await latest.idle();
    expect(started).toEqual([1, 2]);
  });

  test('cancel drops the pending request', async () => {
    const { latest, started, finishNext } = controlled();
    latest.request(1);
    latest.request(2);
    latest.cancel();
    await finishNext();
    expect(started).toEqual([1]);
    expect(latest.busy).toBe(false);
  });
});
