import { describe, expect, it } from 'vitest';
import { formatDistanceM, formatSpeedKph, formatTemperatureC } from './format';

describe('measurement formatting', () => {
  it('formats canonical metric values', () => {
    expect(formatSpeedKph(36, 'metric')).toBe('36.0 km/h');
    expect(formatDistanceM(10_000, 'metric')).toBe('10.00 km');
    expect(formatTemperatureC(20, 'metric')).toBe('20.0 °C');
  });

  it('converts display values to imperial without changing stored values', () => {
    expect(formatSpeedKph(36, 'imperial')).toBe('22.4 mph');
    expect(formatDistanceM(1609.344, 'imperial')).toBe('1.00 mi');
    expect(formatTemperatureC(20, 'imperial')).toBe('68.0 °F');
  });

  it('uses a stable placeholder for absent values', () => {
    expect(formatSpeedKph(null, 'metric')).toBe('—');
    expect(formatDistanceM(undefined, 'imperial')).toBe('—');
    expect(formatTemperatureC(null, 'metric')).toBe('—');
  });
});
