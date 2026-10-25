import { describe, it, expect } from 'vitest';
import { dashboardDomain, dashboardRangeValue, validCustomDashboard } from './custom-dashboard';

const reading = {
  id: 'one',
  size: 'compact',
  kind: 'reading',
  field: 'motorCurrentA',
  presentation: 'gauge',
  range: { min: -100, max: 100 },
};
describe('dashboard contract', () => {
  it('accepts empty, duplicate readings with independent IDs, specialised visuals and controls', () => {
    expect(validCustomDashboard({ version: 1, widgets: [] })).toBe(true);
    expect(
      validCustomDashboard({
        version: 1,
        widgets: [
          reading,
          { ...reading, id: 'two', presentation: 'trend' },
          { id: 'three', kind: 'control', control: 'headlights', size: 'medium' },
          { id: 'four', kind: 'visual', visual: 'battery', size: 'large' },
        ],
      })
    ).toBe(true);
  });
  it('rejects unknown shapes, excessive capacity, duplicate IDs, bad domains and incompatible combinations', () => {
    for (const value of [
      null,
      {},
      { version: 2, widgets: [] },
      { version: 1, widgets: [], extra: true },
      ...[
        [reading, reading],
        Array.from({ length: 25 }, (_, i) => ({ ...reading, id: `w${i}` })),
        [{ ...reading, id: 'x'.repeat(65) }],
        [{ ...reading, field: 'headlights' }],
        [{ ...reading, presentation: 'unknown' }],
        [{ ...reading, range: { min: 100, max: -100 } }],
        [{ ...reading, range: { min: -2001, max: 2000 } }],
        [{ ...reading, range: { min: NaN, max: 2000 } }],
        [{ ...reading, range: { min: -100, max: 100, extra: 1 } }],
        [{ ...reading, field: 'elapsedMs', range: undefined }],
      ].map((widgets) => ({ version: 1, widgets })),
    ])
      expect(validCustomDashboard(value), JSON.stringify(value)).toBe(false);
  });
  it('round-trips bounded independent dimensions alongside existing named sizes', () => {
    for (const columns of [1, 2, 3, 4])
      for (const rows of [1, 2, 3, 4]) {
        const dashboard = {
          version: 1,
          widgets: [
            { ...reading, size: { columns, rows } },
            { ...reading, id: 'existing', size: 'medium' },
          ],
        };
        expect(validCustomDashboard(JSON.parse(JSON.stringify(dashboard)))).toBe(true);
      }
    for (const size of [
      null,
      [],
      {},
      { columns: 1 },
      { rows: 1 },
      { columns: 0, rows: 1 },
      { columns: 5, rows: 1 },
      { columns: 1, rows: 0 },
      { columns: 1, rows: 5 },
      { columns: 1.5, rows: 1 },
      { columns: 1, rows: NaN },
      { columns: '1', rows: 1 },
      { columns: 1, rows: 1, extra: true },
    ])
      expect(validCustomDashboard({ version: 1, widgets: [{ ...reading, size }] }), JSON.stringify(size)).toBe(false);
  });
  it('keeps signed domains and converts ranges without changing physical meaning', () => {
    expect(dashboardDomain('motorCurrentA')?.initial).toEqual({ min: -100, max: 100 });
    expect(dashboardDomain('elapsedMs')).toBeNull();
    expect(dashboardDomain('maxSpeedKph')).toEqual(dashboardDomain('speedKph'));
    expect(dashboardRangeValue('maxSpeedKph', 60, 'imperial')).toBeCloseTo(37.28226);
    expect(
      validCustomDashboard({ version: 1, widgets: [{ ...reading, field: 'maxSpeedKph', range: { min: 0, max: 60 } }] })
    ).toBe(true);
    expect(dashboardRangeValue('motorTempC', 100, 'imperial')).toBe(212);
    expect(dashboardRangeValue('motorTempC', 212, 'imperial', true)).toBe(100);
    expect(
      dashboardRangeValue('speedKph', dashboardRangeValue('speedKph', 60, 'imperial'), 'imperial', true)
    ).toBeCloseTo(60);
  });
});
