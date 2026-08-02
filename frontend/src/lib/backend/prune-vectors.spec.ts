import { describe, expect, it } from 'vitest';
import { selectPruneRows } from './session-reference';
import { PRUNE_VECTORS } from './prune-vectors';

describe('immutable pruning vectors', () => {
  it('retains protected, endpoint, event, first-field, and extrema evidence deterministically', () => {
    const vector = PRUNE_VECTORS[0]!;
    const eventSources = new Set(vector.events.map((event) => event.sourceSampleSequence));
    const retained = selectPruneRows(
      vector.samples,
      vector.fields,
      5,
      new Set(vector.protectedSequences),
      eventSources
    );
    expect(Object.isFrozen(PRUNE_VECTORS)).toBe(true);
    expect(Object.isFrozen(vector.samples)).toBe(true);
    expect(retained).toEqual([...retained].sort((left, right) => left - right));
    expect(retained).toEqual(expect.arrayContaining([0, 23, 5, 12, 13]));
    expect(retained).toEqual(vector.expectedRetainedByResolution[5]);
    expect(retained.length).toBeLessThanOrEqual(vector.samples.length);
  });

  it('keeps revisions and recorded/stored accounting explicit across repeated coarser pruning', () => {
    const vector = PRUNE_VECTORS[0]!;
    const eventSources = new Set(vector.events.map((event) => event.sourceSampleSequence));
    const fine = selectPruneRows(vector.samples, vector.fields, 1, new Set(vector.protectedSequences), eventSources);
    const coarse = selectPruneRows(vector.samples, vector.fields, 5, new Set(fine), eventSources);
    expect(vector.recordedSampleCount).toBe(24);
    expect(vector.afterRevision).toBe(vector.beforeRevision + 1);
    expect(coarse).toEqual(fine);
  });
});
