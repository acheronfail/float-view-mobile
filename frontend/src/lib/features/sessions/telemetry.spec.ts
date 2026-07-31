import { describe, expect, it } from 'vitest';
import { extensionHistoricalFields, seriesPolyline, seriesValues, sessionEventText } from './telemetry';

describe('historical telemetry presentation', () => {
  it('preserves nullability while producing bounded chart input', () => {
    const points = [
      { elapsedMs: 0, values: { fieldWeakeningCurrentA: null } },
      { elapsedMs: 1_000, values: { fieldWeakeningCurrentA: 4 } },
      { elapsedMs: 2_000, values: { fieldWeakeningCurrentA: 8 } },
    ];
    expect(seriesValues(points, 'fieldWeakeningCurrentA')).toEqual([
      { elapsedMs: 1_000, value: 4 },
      { elapsedMs: 2_000, value: 8 },
    ]);
    expect(seriesPolyline(points, 'fieldWeakeningCurrentA')).toBe('0,45 320,5');
  });

  it('renders typed transitions as readable annotations', () => {
    expect(sessionEventText({ elapsedMs: 5_000, kind: 'stopCondition', value: 'switchHalf' })).toBe(
      'Stop Condition: Switch Half'
    );
  });

  it('surfaces negotiated historical fields without duplicating stable fields', () => {
    expect(
      extensionHistoricalFields([
        {
          elapsedMs: 0,
          values: { speedKph: 4, 'future.z': 2, 'future.a': 1 },
        },
      ])
    ).toEqual([
      { key: 'future.a', label: 'future.a', unit: 'source unit' },
      { key: 'future.z', label: 'future.z', unit: 'source unit' },
    ]);
  });
});
