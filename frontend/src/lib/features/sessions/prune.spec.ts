import { describe, expect, it } from 'vitest';
import { formatPruneBytes, hasCoarserPruneResolution, suggestedPruneResolution, validPruneResolution } from './prune';

describe('session prune presentation', () => {
  it('suggests a useful first prune and progressively coarser follow-ups', () => {
    expect(suggestedPruneResolution(null)).toBe(5);
    expect(suggestedPruneResolution(5)).toBe(10);
    expect(suggestedPruneResolution(200)).toBe(300);
    expect(hasCoarserPruneResolution(150)).toBe(true);
    expect(hasCoarserPruneResolution(151)).toBe(false);
  });

  it('validates the backend range and nested resolution requirement', () => {
    expect(validPruneResolution(5, null)).toBe(true);
    expect(validPruneResolution(0, null)).toBe(false);
    expect(validPruneResolution(301, null)).toBe(false);
    expect(validPruneResolution(10, 5)).toBe(true);
    expect(validPruneResolution(6, 5)).toBe(false);
    expect(validPruneResolution(5, 5)).toBe(false);
  });

  it('formats reclaimable storage estimates', () => {
    expect(formatPruneBytes(null)).toBe('Storage estimate unavailable');
    expect(formatPruneBytes(512)).toBe('Estimated space recoverable: 512 B');
    expect(formatPruneBytes(2_048)).toBe('Estimated space recoverable: 2.0 KB');
    expect(formatPruneBytes(2 * 1_024 * 1_024)).toBe('Estimated space recoverable: 2.0 MB');
  });
});
