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
    expect(result.paths).toEqual(['M 0.00 88.40 L 50.00 88.40 L 100.00 88.40']);
    expect(result.bands[0]).toContain('50.00 2.00');
    expect(result.bands[0]).toContain('50.00 98.00');
    expect(spike.values).toEqual([0, 10, 0, 0]);
    const group = sessionChartGroups([{ name: 'motorTempC' }])[0];
    expect(chartExtrema(group, { motorTempC: spike }, new Set(), options.viewport)).toEqual({ min: 0, max: 10 });
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
