import { describe, expect, it } from 'vitest';
import {
  compactElapsed,
  compactViewport,
  originalElapsed,
  originalViewport,
  recordingTimeline,
} from './recording-time';
import { buildPathSegments, pointAtElapsed } from './history-geometry';
import { historyTrend } from './history-trend';
import { chartMarkerGroups } from './marker-groups';
import { panViewport, pinchViewport } from './viewport';
import type { SessionRecordingBreak } from '$lib/backend/contract';

const breaks: SessionRecordingBreak[] = [[2000, 8000, 'pause']];
const viewport = { startElapsedMs: 0, endElapsedMs: 10000 };
const options = { width: 100, height: 50, viewport, recordingBreaks: breaks, domain: { min: 0, max: 10 } };
const column = {
  elapsedMs: [0, 2000, 8000, 10000],
  sequence: [1, 2, 3, 4],
  values: [0, 10, 5, 0],
  generations: [0, 0, 1, 1],
};

describe('confirmed recording time', () => {
  it('keeps unknown and connection-loss gaps at their original width alongside deliberate pauses', () => {
    const mixed: SessionRecordingBreak[] = [
      [1000, 2000, 'connectionLoss'],
      [3000, 5000, 'pause'],
      [6000, 8000, 'unknown'],
    ];
    const axis = recordingTimeline(viewport, mixed);
    expect(axis.duration).toBe(8000);
    expect(axis.recordingSeparators).toEqual([{ start: 3000, end: 5000, ratio: 0.375 }]);
    expect(axis.ratioAt(1500)).toBe(0.1875);
    expect(axis.ratioAt(7000)).toBe(0.625);
    expect(axis.elapsedAt(0.625)).toBe(7000);
    expect(compactElapsed(10000, mixed)).toBe(8000);
    expect(originalElapsed(8000, mixed)).toBe(10000);
    const unknown = [[2000, 8000, 'unknown']] as SessionRecordingBreak[];
    expect(recordingTimeline(viewport, unknown).recordingSeparators).toEqual([]);
    expect(buildPathSegments(column, { ...options, recordingBreaks: unknown })[1]).toMatch(/^M 80.00 /);
    const filtered = recordingTimeline(viewport, mixed, [viewport]);
    expect(filtered.duration).toBe(8000);
    expect(filtered.recordingSeparators).toHaveLength(1);
  });
  it('collapses only confirmed intervals and keeps original times reversible', () => {
    const axis = recordingTimeline(viewport, breaks);
    expect(axis.duration).toBe(4000);
    expect(axis.ratioAt(2000)).toBe(0.5);
    expect(axis.ratioAt(8000)).toBe(0.5);
    expect(axis.ratioAt(5000)).toBeNull();
    expect(axis.elapsedAt(0.75)).toBe(9000);
    expect(axis.elapsedAt(0)).toBe(0);
    expect(axis.elapsedAt(1)).toBe(10000);
    for (const time of [0, 1000, 8000, 9000, 10000])
      expect(originalElapsed(compactElapsed(time, breaks), breaks)).toBe(time);
    expect(axis.recordingSeparators).toEqual([{ start: 2000, end: 8000, ratio: 0.5 }]);
    expect(recordingTimeline(viewport).duration).toBe(10000);
  });

  it('projects traces, inspection and markers consistently without bridging', () => {
    expect(buildPathSegments(column, options)).toEqual(['M 0.00 48.00 L 50.00 2.00', 'M 50.00 25.00 L 100.00 48.00']);
    expect(historyTrend(column, options, 100).paths).toEqual(buildPathSegments(column, options));
    expect(pointAtElapsed(column, 9000, options.domain, options)?.x).toBe(75);
    expect(pointAtElapsed(column, 5000, options.domain, options)).toBeNull();
    expect(chartMarkerGroups([{ sequence: 1, elapsedMs: 9000 }], viewport, 100, undefined, breaks)[0]?.percent).toBe(
      75
    );
    expect(buildPathSegments(column, { ...options, recordingBreaks: [] })[1]).toMatch(/^M 80.00 /);
  });

  it('composes map selections with recording breaks', () => {
    const intervals = [
      { startElapsedMs: 1000, endElapsedMs: 9000 },
      { startElapsedMs: 9500, endElapsedMs: 10000 },
    ];
    const axis = recordingTimeline(viewport, breaks, intervals);
    expect(axis.duration).toBe(2500);
    expect(axis.ratioAt(8000)).toBe(0.4);
    expect(axis.elapsedAt(0.6)).toBe(8500);
    expect(axis.ratioAt(9250)).toBeNull();
    expect(axis.separators).toEqual([0.8]);
    expect(axis.recordingSeparators[0]?.ratio).toBe(0.4);
  });

  it('handles offscreen points, clipped breaks and wholly missing viewports', () => {
    const axis = recordingTimeline({ startElapsedMs: 1000, endElapsedMs: 9000 }, breaks);
    expect(axis.ratioAt(0)).toBe(-0.5);
    expect(axis.ratioAt(10000)).toBe(1.5);
    const missing = recordingTimeline({ startElapsedMs: 3000, endElapsedMs: 7000 }, breaks);
    expect(missing.duration).toBe(0);
    expect(missing.ratioAt(0)).toBeNull();
    expect(missing.elapsedAt(0.5)).toBeNull();
    const missingOptions = { ...options, viewport: { startElapsedMs: 3000, endElapsedMs: 7000 } };
    expect(buildPathSegments(column, missingOptions)).toEqual([]);
    expect(pointAtElapsed(column, 0, options.domain, missingOptions)).toBeNull();
    expect(recordingTimeline({ startElapsedMs: 8001, endElapsedMs: 10000 }, breaks).recordingSeparators).toEqual([]);
  });

  it('zooms and pans by recorded time across the collapsed seam', () => {
    const compact = compactViewport(viewport, breaks);
    const zoomed = pinchViewport(compact, 100, 200, 0.75, 4000);
    expect(originalViewport(zoomed, breaks)).toEqual({ startElapsedMs: 1500, endElapsedMs: 9500 });
    expect(originalViewport(panViewport(zoomed, -1000, 4000), breaks)).toEqual({
      startElapsedMs: 500,
      endElapsedMs: 8500,
    });
  });
});
