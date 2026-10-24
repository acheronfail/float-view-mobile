import { describe, expect, it } from 'vitest';
import { validRecordingBreaks } from './recording-breaks';
import { MAX_MESSAGE_BYTES } from './contract';

describe('recording break metadata', () => {
  it('keeps 1000 maximum-width reason-bearing intervals below the envelope limit', () => {
    const end = Number.MAX_SAFE_INTEGER;
    const rows = Array.from({ length: 1000 }, (_, i) => [end - 2000 + i * 2, end - 1999 + i * 2, 'connectionLoss']);
    expect(validRecordingBreaks(rows, end)).toBe(true);
    // Reserve 12 KiB for the surrounding metadata and request envelope.
    expect(JSON.stringify({ recordingBreaks: rows }).length + 12 * 1024).toBeLessThan(MAX_MESSAGE_BYTES);
  });
  it('accepts ordered committed endpoints and rejects malformed or oversized collections', () => {
    expect(validRecordingBreaks([], 100)).toBe(true);
    expect(
      validRecordingBreaks(
        [
          [0, 10, 'pause'],
          [10, 20, 'connectionLoss'],
          [20, 30, 'unknown'],
        ],
        30
      )
    ).toBe(true);
    for (const invalid of [
      undefined,
      [[1]],
      [[0, 10]],
      [[0, 10, null]],
      [[0, 10, 'guessedPause']],
      [[-1, 2, 'pause']],
      [[2, 2, 'pause']],
      [[1, 2001, 'pause']],
      [
        [0, 10, 'unknown'],
        [9, 20, 'pause'],
      ],
      [[0, 1.5, 'pause']],
      Array.from({ length: 1001 }, (_, i) => [i * 2, i * 2 + 1, 'pause']),
    ])
      expect(validRecordingBreaks(invalid, 2000)).toBe(false);
  });
});
