import { describe, expect, it } from 'vitest';
import type { LiveSample } from '$lib/backend/contract';
import { liveChartGeometry, retainLivePoints, liveChartPointLimit } from './live-charts';
const point = (time: number, speedKph: number | null) => ({ time, sample: { speedKph } as LiveSample });
describe('live chart window', () => {
  it('expires old samples and bounds the buffer', () => {
    expect(retainLivePoints([point(0, 1), point(10001, 2)], 40000)).toEqual([point(10001, 2)]);
    expect(
      retainLivePoints(
        Array.from({ length: 2000 }, (_, time) => point(time, 1)),
        2000
      )
    ).toHaveLength(liveChartPointLimit);
  });
  it('scrolls without extending the last value and breaks missing data and gaps', () => {
    const points = [point(1000, 10), point(1100, 11), point(1200, null), point(1300, 12), point(4000, 15)];
    const first = liveChartGeometry(points, ['speedKph'], 'metric', 4000);
    expect(first.paths[0].match(/M/g)).toHaveLength(3);
    expect(first.paths[0]).toContain('L');
    expect(first.paths[0]).toContain('M1000.00,');
    expect(liveChartGeometry(points, ['speedKph'], 'metric', 7000).paths[0]).toContain('M900.00,');
  });
  it('converts display units and keeps signed current with a shared axis', () => {
    const sample = { speedKph: 16.09344, motorCurrentA: -12, batteryCurrentA: 20 } as LiveSample;
    const metric = liveChartGeometry([{ time: 0, sample }], ['speedKph'], 'metric', 0);
    const imperial = liveChartGeometry([{ time: 0, sample }], ['speedKph'], 'imperial', 0);
    expect(imperial.high).toBeLessThan(metric.high);
    const current = liveChartGeometry([{ time: 0, sample }], ['motorCurrentA', 'batteryCurrentA'], 'metric', 0);
    expect(current.low).toBeLessThan(-12);
    expect(current.high).toBeGreaterThan(20);
    expect(current.paths).toHaveLength(2);
  });
});
