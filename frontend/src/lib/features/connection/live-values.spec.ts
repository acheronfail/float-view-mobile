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

describe('live value colours', () => {
  it('uses footpad state rather than ADC voltage and alerts only above the speed threshold', () => {
    const atRest = { ...sample, footpadState: 'left', speedKph: 0 } as LiveSample;
    expect(liveValueColor('footpadAdc1V', 0.1, atRest)).toBe('text-success');
    expect(liveValueColor('footpadAdc2V', 3.3, atRest)).toBe('text-muted');
    expect(liveValueColor('footpadAdc2V', 3.3, { ...atRest, speedKph: 2 })).toBe('text-muted');
    expect(liveValueColor('footpadAdc2V', 3.3, { ...atRest, speedKph: 2.1 })).toBe('text-danger');
    expect(liveValueColor('footpadAdc1V', 0.1, { ...atRest, speedKph: 20 })).toBe('text-success');
    expect(liveValueColor('footpadAdc2V', null, { ...atRest, speedKph: 20 })).toBe('text-foreground');
    expect(liveValueColor('footpadAdc2V', 3.3, { ...atRest, footpadState: 'both' })).toBe('text-success');
  });
  it('keeps temperature meaning consistent across display units', () => {
    for (const [celsius, color] of [
      [30, 'text-success'],
      [45, 'text-warning'],
      [60, 'text-danger'],
    ] as const) {
      expect(liveValueColor('motorTempC', celsius)).toBe(color);
      expect(liveValueColor('controllerTempC', celsius * 1.8 + 32, sample, 'imperial')).toBe(color);
    }
  });
  it('colours percentages and electrical directions without guessing voltage limits', () => {
    expect(liveValueColor('estimatedBatteryLevel', 20)).toBe('text-danger');
    expect(liveValueColor('batteryLevel', 35)).toBe('text-warning');
    expect(liveValueColor('batteryLevel', 50)).toBe('text-success');
    expect(liveValueColor('batteryCurrentA', 10)).toBe('text-power-draw');
    expect(liveValueColor('batteryCurrentA', -10)).toBe('text-success');
    expect(liveValueColor('batteryCurrentA', 0)).toBe('text-foreground');
    expect(liveValueColor('packVoltageV', 60)).toBe('text-foreground');
    expect(liveValueColor('dutyCycle', 40)).toBe('text-duty');
    expect(liveValueColor('wattHoursDrawnWh', 100)).toBe('text-power-draw');
    expect(liveValueColor('wattHoursChargedWh', 10)).toBe('text-success');
  });
});
