import { describe, expect, it } from 'vitest';
import { liveValue, liveValueCatalog, liveValueColor, liveValueText } from './live-values';
import {
  defaultLiveWidgetSlots,
  liveWidgetFields,
  type ControllerCurrentLimits,
  type LiveSample,
} from '$lib/backend/contract';
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
    expect(defaultLiveWidgetSlots).toEqual(['empty', 'powerW', 'elapsedMs', 'odometerM', 'distanceM']);
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

describe('load widget', () => {
  const limits: ControllerCurrentLimits = {
    motorCurrentMaxA: 100,
    motorCurrentMinA: -50,
    batteryCurrentMaxA: 50,
    batteryCurrentMinA: -20,
    batterySeriesCells: 18,
  };
  it.each([
    [0.9, 60, 10, 90, 'Duty', 'text-duty'],
    [0.2, 60, 10, 60, 'Motor', 'text-motor'],
    [0.2, -10, -15, 75, 'Battery', 'text-battery-current'],
  ] as const)(
    'matches the gauge source and percentage',
    (dutyCycle, motorCurrentA, batteryCurrentA, value, label, color) => {
      const readingSample = { ...sample, dutyCycle, motorCurrentA, batteryCurrentA };
      expect(liveValue('driveLoad', readingSample, 'metric', null, limits)).toMatchObject({ value, label, unit: '%' });
      expect(liveValueColor('driveLoad', value, readingSample, 'metric', limits)).toBe(color);
    }
  );
  it('keeps unavailable load unavailable and supports duty without limits', () => {
    expect(liveValue('driveLoad', null, 'metric').value).toBeNull();
    expect(liveValue('driveLoad', { ...sample, dutyCycle: 0.4 }, 'imperial').value).toBe(40);
  });
});

describe('BMS boxes and highest temperature', () => {
  const now = Date.parse('2026-10-13T00:00:00Z');
  const bms = {
    status: 'attached',
    connection: { kind: 'can', canId: 3 },
    candidates: [],
    latestSnapshot: {
      observedAt: new Date(now).toISOString(),
      totalVoltageV: 75,
      inputCurrentA: -2,
      chargeVoltageV: 80,
      stateOfCharge: 0.75,
      stateOfHealth: 0.98,
      cellVoltagesV: [4.1, 4.05, 4.08],
      temperaturesC: [31, 47],
      highestCellTemperatureC: 44,
      icTemperatureC: 40,
      humiditySensorTemperatureC: 35,
      humidityPercent: 22,
      inputIcCurrentA: 0,
      sessionAmpHoursAh: 0,
      sessionWattHoursWh: 0,
      balancing: [],
      reportedCanId: 3,
      lifetimeChargeAmpHoursAh: 0,
      lifetimeChargeWattHoursWh: 0,
      lifetimeDischargeAmpHoursAh: 0,
      lifetimeDischargeWattHoursWh: 0,
      pressurePa: 0,
      dataVersion: 1,
      status: null,
    },
  } as import('$lib/backend/contract').BmsState;
  const reading = (field: import('$lib/backend/contract').LiveWidgetField, state = bms, time = now) =>
    liveValue(field, { ...sample, controllerTempC: 38, motorTempC: 45 }, 'metric', null, null, state, time);

  it('uses BMS fractions, signed current, cell extrema and all battery probes', () => {
    expect(reading('bmsStateOfCharge').value).toBe(75);
    expect(reading('bmsStateOfHealth').value).toBe(98);
    expect(reading('bmsTotalVoltageV').value).toBe(75);
    expect(reading('bmsInputCurrentA').value).toBe(-2);
    expect(reading('bmsChargeVoltageV').value).toBe(80);
    expect(reading('bmsMinCellVoltageV').value).toBe(4.05);
    expect(reading('bmsMaxCellVoltageV').value).toBe(4.1);
    expect(reading('bmsCellVoltageSpreadV').value).toBeCloseTo(0.05);
    expect(reading('bmsHighestCellTempC').value).toBe(47);
    expect(reading('bmsIcTempC').value).toBe(40);
    expect(reading('bmsHumidityPercent').value).toBe(22);
    expect(reading('highestTempC')).toMatchObject({ value: 47, label: 'Battery temp' });
  });

  it.each(['not_found', 'discovering', 'unavailable', 'ambiguous'] as const)('falls back when BMS is %s', (status) => {
    const unavailable = { ...bms, status };
    for (const field of liveWidgetFields.filter((field) => field.startsWith('bms')))
      expect(reading(field, unavailable).value).toBeNull();
    expect(reading('highestTempC', unavailable)).toMatchObject({ value: 45, label: 'Motor temp' });
  });

  it('expires stale, future and invalid BMS timestamps, and handles empty cells', () => {
    for (const observedAt of [new Date(now - 5000).toISOString(), new Date(now + 1).toISOString(), 'invalid']) {
      const stale = { ...bms, latestSnapshot: { ...bms.latestSnapshot!, observedAt } };
      expect(reading('bmsStateOfCharge', stale).value).toBeNull();
      expect(reading('highestTempC', stale)).toMatchObject({ value: 45, label: 'Motor temp' });
    }
    expect(reading('bmsStateOfCharge', bms, now + 4999).value).toBe(75);
    expect(
      reading('bmsMinCellVoltageV', { ...bms, latestSnapshot: { ...bms.latestSnapshot!, cellVoltagesV: [] } }).value
    ).toBeNull();
    expect(liveValue('highestTempC', null, 'metric', null, null, null, now).value).toBeNull();
  });

  it('identifies every winning sensor, switches immediately and converts the winner to Fahrenheit', () => {
    for (const [key, label] of [
      ['icTemperatureC', 'BMS temp'],
      ['humiditySensorTemperatureC', 'Humidity temp'],
    ] as const) {
      const state = { ...bms, latestSnapshot: { ...bms.latestSnapshot!, [key]: 70 } };
      expect(reading('highestTempC', state)).toMatchObject({ value: 70, label });
    }
    expect(
      liveValue('highestTempC', { ...sample, controllerTempC: 80 }, 'imperial', null, null, bms, now)
    ).toMatchObject({ value: 176, label: 'ESC temp', unit: '°F' });
    expect(liveValue('highestTempC', { ...sample, controllerTempC: -10, motorTempC: -5 }, 'metric')).toMatchObject({
      value: -5,
      label: 'Motor temp',
    });
    expect(liveValue('highestTempC', { ...sample, controllerTempC: NaN, motorTempC: 0 }, 'metric')).toMatchObject({
      value: 0,
      label: 'Motor temp',
    });
    expect(liveValueColor('highestTempC', 176, sample, 'imperial')).toBe('text-danger');
  });
});
