import { describe, expect, it } from 'vitest';
import { historyTrend } from './history-trend';
import { buildPathSegments } from './history-geometry';
import { chartExtrema } from './chart-extrema';
import { sessionChartGroups } from './session-fields';

const options = {
  width: 100,
  height: 100,
  viewport: { startElapsedMs: 0, endElapsedMs: 10000 },
  domain: { min: 0, max: 10 },
};
const spike = {
  elapsedMs: [0, 1000, 2000, 10000],
  sequence: [1, 2, 3, 4],
  values: [0, 10, 0, 0],
  generations: [0, 0, 0, 0],
};

describe('history trend', () => {
  it('weights by time and preserves a brief spike in the band and original extrema', () => {
    const result = historyTrend(spike, options, 8);
    expect(result.smoothed).toBe(true);
    // The two-second triangle averages to 1 over the ten-second range.
    expect(result.paths).toEqual([
      'M 0.00 88.40 C 16.67 88.40 33.33 88.40 50.00 88.40 C 66.67 88.40 83.33 88.40 100.00 88.40',
    ]);
    expect(result.bands[0]).toContain('50.00 2.00');
    expect(result.bands[0]).toContain('50.00 98.00');
    expect(spike.values).toEqual([0, 10, 0, 0]);
    const group = sessionChartGroups([{ name: 'motorTempC' }])[0];
    expect(chartExtrema(group, { motorTempC: spike }, new Set(), options.viewport)).toEqual({ min: 0, max: 10 });
  });

  it('restores every retained reading without bands when smoothing is disabled', () => {
    expect(historyTrend(spike, options, 8, false)).toEqual({
      paths: buildPathSegments(spike, options),
      bands: [],
      smoothed: false,
    });
  });

  it('curves changing trends with continuous tangents and no invented extrema', () => {
    const result = historyTrend(
      {
        elapsedMs: [0, 2000, 4000, 6000, 8000, 10000],
        sequence: [0, 1, 2, 3, 4, 5],
        values: [0, 2, 8, 4, 2, 3],
        generations: [0, 0, 0, 0, 0, 0],
      },
      options,
      40
    );
    expect(result.bands).toHaveLength(1);
    const path = result.paths[0];
    expect(path).not.toContain(' L ');
    const coordinates = path.match(/-?\d+\.\d+/g)!.map(Number);
    let left = coordinates.slice(0, 2);
    let previousTangent: number | null = null;
    for (let index = 2; index < coordinates.length; index += 6) {
      const [cx1, cy1, cx2, cy2, rx, ry] = coordinates.slice(index, index + 6);
      for (const controlY of [cy1, cy2]) {
        expect(controlY).toBeGreaterThanOrEqual(Math.min(left[1], ry));
        expect(controlY).toBeLessThanOrEqual(Math.max(left[1], ry));
      }
      const startTangent = (cy1 - left[1]) / (cx1 - left[0]);
      if (previousTangent !== null) expect(startTangent).toBeCloseTo(previousTangent, 2);
      previousTangent = (ry - cy2) / (rx - cx2);
      left = [rx, ry];
    }
  });

  it('restores retained detail when zooming reduces the smoothing window below one second', () => {
    const zoom = { ...options, viewport: { startElapsedMs: 0, endElapsedMs: 2000 } };
    expect(historyTrend(spike, zoom, 320)).toEqual({
      paths: buildPathSegments(spike, zoom),
      bands: [],
      smoothed: false,
    });
  });

  it('never joins trends or bands across nulls, generations, or excluded map intervals', () => {
    const column = {
      elapsedMs: [0, 1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000],
      sequence: [1, 2, 3, 4, 5, 6, 7, 8, 9],
      values: [0, 1, 2, 3, null, 5, 6, 7, 8],
      generations: Array(9).fill(0),
    };
    expect(historyTrend(column, options, 8).bands).toHaveLength(2);
    const noNull = { ...column, values: column.values.map((v) => v ?? 4), generations: [0, 0, 0, 0, 1, 1, 1, 1, 1] };
    expect(historyTrend(noNull, options, 8).bands).toHaveLength(2);
    const spatial = historyTrend(
      { ...noNull, generations: Array(9).fill(0) },
      {
        ...options,
        intervals: [
          { startElapsedMs: 0, endElapsedMs: 3000 },
          { startElapsedMs: 5000, endElapsedMs: 8000 },
        ],
      },
      8
    );
    expect(spatial.bands).toHaveLength(2);
    expect(spatial.paths[0]).not.toContain('100.00');
    expect(spatial.paths[1]).toMatch(/^M 50.00 /);
  });

  it('keeps isolated readings drawable without inventing bands', () => {
    const result = historyTrend({ ...spike, generations: [0, 1, 2, 3] }, options, 8);
    expect(result.smoothed).toBe(false);
    expect(result.bands).toEqual([]);
    expect(result.paths).toHaveLength(4);
    expect(result.paths.every((path) => path.endsWith('l 0 0'))).toBe(true);
  });
});
