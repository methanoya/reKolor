// Unit tests for `limits.ts` (see `palette.test.ts` for how these test files work).
// `toMatch(/regex/)` checks a string against a regular expression.
import { describe, expect, test } from 'vitest';
import { LIMITS, fileLimitError, imageLimitError } from './limits';

describe('limits', () => {
  test('images within the limits pass', () => {
    expect(imageLimitError(1, 1)).toBeUndefined();
    expect(imageLimitError(16_384, 1_464)).toBeUndefined(); // 23.99 MP
    expect(imageLimitError(6_000, 4_000)).toBeUndefined(); // exactly 24 MP
  });

  test('too wide, too tall, too many pixels or empty images fail', () => {
    expect(imageLimitError(16_385, 1)).toMatch(/at most 16384/);
    expect(imageLimitError(1, 16_385)).toMatch(/at most 16384/);
    expect(imageLimitError(6_001, 4_000)).toMatch(/megapixels/);
    expect(imageLimitError(0, 10)).toMatch(/empty/);
    expect(imageLimitError(Number.NaN, 10)).toMatch(/empty/);
  });

  test('file size', () => {
    expect(fileLimitError(LIMITS.fileBytes)).toBeUndefined();
    expect(fileLimitError(LIMITS.fileBytes + 1)).toMatch(/MB/);
  });
});
