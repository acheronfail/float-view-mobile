import { describe, expect, it } from 'vitest';
import { buildChartTooltipEntries, formatChartElapsedTime } from './chart-tooltip';
import { sessionChartGroups } from './session-fields';

describe('chart tooltip elapsed time', () => {
  it.each([
    [0, '0s'],
    [300, '0.3s'],
    [50_000, '50s'],
    [60_000, '1m'],
    [60_300, '1m 0.3s'],
    [3_600_000, '1h'],
    [3_601_000, '1h 1s'],
    [3_660_000, '1h 1m'],
    [8_368_300, '2h 19m 28.3s'],
    [59_960, '1m'],
    [3_599_960, '1h'],
    [90_000_000, '25h'],
  ])('formats %i ms as %s', (elapsedMs, expected) => {
    expect(formatChartElapsedTime(elapsedMs)).toBe(expected);
  });
});

describe('chart tooltip entries', () => {
  it('combines visible lines at one elapsed x-position', () => {
    const chartGroup = sessionChartGroups([{ name: 'controllerTempC' }, { name: 'motorTempC' }])[0]!;
    const entries = buildChartTooltipEntries(
      chartGroup,
      {
        controllerTempC: {
          elapsedMs: [0, 10],
          sequence: [0, 1],
          values: [30, 32],
          generations: [0, 0],
        },
        motorTempC: {
          elapsedMs: [0, 10],
          sequence: [0, 1],
          values: [28, 31],
          generations: [0, 0],
        },
      },
      new Set<string>(),
      5,
      { min: 20, max: 40 },
      { width: 320, height: 96, viewport: { startElapsedMs: 0, endElapsedMs: 10 } }
    );

    expect(entries.map((entry) => entry.field.name)).toEqual(['controllerTempC', 'motorTempC']);
    expect(entries.map((entry) => entry.point.x)).toEqual([160, 160]);
    expect(entries.map((entry) => entry.point.value)).toEqual([31, 29.5]);
  });

  it('omits hidden or unavailable lines without changing the shared elapsed position', () => {
    const chartGroup = sessionChartGroups([{ name: 'controllerTempC' }, { name: 'motorTempC' }])[0]!;
    const entries = buildChartTooltipEntries(
      chartGroup,
      {
        controllerTempC: {
          elapsedMs: [0, 10],
          sequence: [0, 1],
          values: [30, 32],
          generations: [0, 0],
        },
      },
      new Set(['controllerTempC']),
      5,
      { min: 20, max: 40 },
      { width: 320, height: 96, viewport: { startElapsedMs: 0, endElapsedMs: 10 } }
    );

    expect(entries).toEqual([]);
  });
});
