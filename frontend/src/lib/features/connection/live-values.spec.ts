import { describe, expect, it } from 'vitest';
import { liveValue, liveValueCatalog, liveValueColor, liveValueText } from './live-values';
import { liveWidgetFields, type LiveSample } from '$lib/backend/contract';
const sample = {
  packVoltageV: 75,
  batteryCurrentA: -4,
  speedKph: 20,
  distanceM: 1609.344,
  motorTempC: 0,
  fieldWeakeningCurrentA: 0.7,
} as LiveSample;
describe('live value selection', () => {
  it('offers every allowed widget field', () => {
    expect(Object.keys(liveValueCatalog)).toEqual([...liveWidgetFields]);
  });
  it('keeps sign, units and missing values', () => {
    expect(liveValue('powerW', sample, 'metric').value).toBe(-300);
    expect(liveValue('powerW', { ...sample, packVoltageV: null }, 'metric').value).toBeNull();
    expect(liveValue('distanceM', sample, 'imperial')).toMatchObject({ value: 1, unit: 'mi' });
    expect(liveValue('motorTempC', sample, 'imperial')).toMatchObject({ value: 32, unit: '°F' });
    expect(liveValue('fieldWeakeningCurrentA', sample, 'metric').value).toBe(0.7);
    expect(liveValue('estimatedBatteryLevel', sample, 'metric', 0.56).value).toBeCloseTo(56);
    expect(liveValue('batteryLevel', sample, 'metric').value).toBeNull();
  });
  it('formats power direction, time and unavailable readings', () => {
    expect(liveValueText('powerW', 900, 0)).toBe('+900');
    expect(liveValueText('powerW', -0.1, 0)).toBe('0');
    expect(liveValueText('elapsedMs', 3_661_000, 0)).toBe('1:01:01');
    expect(liveValueText('powerW', null, 0)).toBe('—');
    expect(liveValueColor('powerW', 900)).toBe('text-power-draw');
    expect(liveValueColor('powerW', -300)).toBe('text-success');
  });
});
