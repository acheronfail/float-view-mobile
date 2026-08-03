import { describe, expect, it } from 'vitest';
import { buildChartTooltipEntries } from './chart-tooltip';
import { sessionChartGroups } from './session-fields';

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
