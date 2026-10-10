import { expect, it } from 'vitest';
import type { SessionRideMarker } from '$lib/backend/contract';
import { chartMarkerGroups } from './marker-groups';
it('keeps sparse markers independent of samples and groups screen collisions', () => {
  const markers = [0, 1, 500, 999, 1001].map(
    (elapsedMs, i) =>
      ({
        sequence: i + 1,
        elapsedMs,
        observedAtMs: elapsedMs,
        operationId: `00000000-0000-4000-8000-${String(i).padStart(12, '0')}`,
        location: null,
      }) satisfies SessionRideMarker
  );
  const groups = chartMarkerGroups(markers, { startElapsedMs: 0, endElapsedMs: 1000 }, 320);
  expect(groups.map((g) => g.count)).toEqual([2, 1, 1]);
  expect(groups.map((g) => g.marker.sequence)).toEqual([1, 3, 4]);
});

it('excludes spatially hidden markers and projects included timestamps by duration', () => {
  const markers = [50, 300, 550].map((elapsedMs, i) => ({
    sequence: i + 1,
    elapsedMs,
    observedAtMs: elapsedMs,
    operationId: `00000000-0000-4000-8000-${String(i).padStart(12, '0')}`,
    location: null,
  }));
  const groups = chartMarkerGroups(markers, { startElapsedMs: 0, endElapsedMs: 600 }, 320, [
    { startElapsedMs: 0, endElapsedMs: 100 },
    { startElapsedMs: 500, endElapsedMs: 600 },
  ]);
  expect(groups.map((group) => [group.marker.elapsedMs, group.percent])).toEqual([
    [50, 25],
    [550, 75],
  ]);
});
