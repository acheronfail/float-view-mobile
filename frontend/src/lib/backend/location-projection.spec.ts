import { describe, expect, it } from 'vitest';
import { csvLocationColumns, nearestLocation } from './location-projection';

const points = [
  { locationSequence: 3, elapsedMs: 2_000, accuracyM: null, latitude: -33.87, longitude: 151.21 },
  { locationSequence: 4, elapsedMs: 4_000, accuracyM: 8.5, latitude: -33.88, longitude: 151.22 },
];

describe('session CSV location projection', () => {
  it('chooses the nearest accepted point and resolves equal-time ties by sequence', () => {
    expect(nearestLocation(3_000, points)).toEqual({ ...points[0] });
    expect(csvLocationColumns(4_000, points)).toEqual({ accuracy: '8.5', longitude: '151.22', latitude: '-33.88' });
  });

  it('leaves all columns blank outside the tolerance and preserves missing accuracy', () => {
    expect(csvLocationColumns(14_001, points)).toEqual({ accuracy: '', longitude: '', latitude: '' });
    expect(csvLocationColumns(2_000, points).accuracy).toBe('');
  });
});
