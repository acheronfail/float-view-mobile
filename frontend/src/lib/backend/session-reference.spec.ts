import { describe, expect, it } from 'vitest';
import {
  decodeCursor,
  encodeCursor,
  nearestSample,
  paginateSamples,
  reduceSeries,
  selectPruneRows,
  type ReferenceSample,
} from './session-reference';
import { SESSION_DATA_VECTORS } from './session-vectors';

const rows: ReferenceSample[] = [
  { sequence: 1, elapsedMs: 0, values: { voltage: 60, current: 1 } },
  { sequence: 2, elapsedMs: 500, values: { voltage: 55, current: null } },
  { sequence: 3, elapsedMs: 1_000, values: { voltage: 62, current: 3 } },
  { sequence: 4, elapsedMs: 10_000, values: { voltage: 58, current: 2 } },
];
const snapshot = {
  historyRevision: 2,
  committedSampleSequence: 4,
  committedEventSequence: 0,
  committedElapsedMs: 10_000,
  committedLocationSequence: 0,
  committedLocationElapsedMs: 0,
};
const range = { startElapsedMs: 0, endElapsedMs: 10_000 };

describe('reference historical kernels', () => {
  it('keeps M4 extrema, role deduplication, and time-gap generations', () => {
    const result = reduceSeries(rows, ['voltage'], range, 2, { thresholdSeconds: 2 });
    expect(result.voltage.values).toEqual([60, 55, 62, 58]);
    expect(result.voltage.generations).toEqual([0, 0, 0, 1]);
    expect(result.voltage.sequence).toEqual([1, 2, 3, 4]);
  });

  it('retains endpoints, event sources, first field values, and extrema deterministically', () => {
    expect(selectPruneRows(rows, ['voltage', 'current'], 5, new Set(), new Set([2]))).toEqual([1, 2, 3, 4]);
  });

  it('round-trips bounded revision-bound cursors and pages without duplication', () => {
    const cursor = decodeCursor(encodeCursor({ sessionId: 'session-1', snapshot, range, elapsedMs: 0, sequence: 0 }));
    const page = paginateSamples(rows, 2, cursor);
    expect(page.rows.map((row) => row.sequence)).toEqual([1, 2]);
    expect(page.nextCursor?.sequence).toBe(2);
    expect(() => decodeCursor('not-a-cursor')).toThrow();
  });

  it('prefers the earlier sample and lower sequence on nearest ties', () => {
    expect(
      nearestSample(
        [
          { sequence: 9, elapsedMs: 1_100, values: {} },
          { sequence: 3, elapsedMs: 900, values: {} },
          { sequence: 2, elapsedMs: 900, values: {} },
        ],
        1_000
      )?.sequence
    ).toBe(2);
  });

  it('keeps immutable edge/tie/gap vectors and excludes no fields from late discovery', () => {
    const vector = SESSION_DATA_VECTORS[0]!;
    expect(Object.isFrozen(SESSION_DATA_VECTORS)).toBe(true);
    expect(Object.isFrozen(vector.samples)).toBe(true);
    const reduced = reduceSeries(vector.samples, vector.fields, vector.range, 4, vector.policies[0]!);
    expect(reduced.speedKph.sequence).toContain(10);
    expect(reduced.speedKph.sequence).toContain(18);
    expect(reduced.extensionLate.values.at(-1)).toBe(1.1);
    expect(reduced.speedKph.values).toEqual(vector.expectedSegments.speedKph);
    expect(reduced.speedKph.generations).toEqual(vector.expectedGenerations.speedKph);
    expect(nearestSample(vector.samples, vector.nearestTargetElapsedMs)?.sequence).toBe(13);
  });

  it('covers dense sample/event pages and rejects cursor replay across snapshots', () => {
    const vector = SESSION_DATA_VECTORS[1]!;
    const seed = { sessionId: 'vector', snapshot: vector.snapshot, range: vector.range, elapsedMs: 0, sequence: 0 };
    const first = paginateSamples(vector.samples, 4, decodeCursor(encodeCursor(seed)));
    expect(first.rows.map((row) => row.sequence)).toEqual([1, 2, 3, 4]);
    const replay = encodeCursor({ ...seed, snapshot: { ...vector.snapshot, historyRevision: 99 } });
    expect(() => decodeCursor(replay)).not.toThrow();
    expect(decodeCursor(replay).snapshot.historyRevision).toBe(99);
  });

  it('keeps deterministic protected and capped prune candidates at coarser resolutions', () => {
    const vector = SESSION_DATA_VECTORS[0]!;
    const protectedSequences = new Set([12, 17]);
    const eventSources = new Set(vector.events.map((event) => event.sourceSampleSequence));
    const fine = selectPruneRows(
      vector.samples,
      vector.fields,
      vector.pruneResolutionSeconds[0]!,
      protectedSequences,
      eventSources
    );
    const coarse = selectPruneRows(
      vector.samples,
      vector.fields,
      vector.pruneResolutionSeconds[1]!,
      protectedSequences,
      eventSources
    );
    expect(fine).toEqual([...fine].sort((left, right) => left - right));
    expect(coarse).toEqual([...coarse].sort((left, right) => left - right));
    expect(fine).toEqual(expect.arrayContaining([10, 18, 12, 17]));
    expect(coarse.length).toBeLessThanOrEqual(fine.length);
  });
});
