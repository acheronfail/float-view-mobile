import { describe, expect, it } from 'vitest';
import type { BmsState, LiveSample } from '$lib/backend/contract';
import {
  liveChartColor,
  liveChartGeometry,
  liveChartGroups,
  liveChartPointLimit,
  liveChartReading,
  retainLivePoints,
} from './live-charts';
const point = (time: number, speedKph: number | null) => ({ time, sample: { speedKph } as LiveSample });
describe('live chart window', () => {
  it('plots gauge load components on a fixed percentage axis with matching colours', () => {
    const currentLimits = {
      motorCurrentMaxA: 100,
      motorCurrentMinA: -50,
      batteryCurrentMaxA: 50,
      batteryCurrentMinA: -20,
      batterySeriesCells: null,
    };
    const sample = {
      dutyCycle: -0.4,
      motorCurrentA: -10,
      filteredMotorCurrentA: 30,
      batteryCurrentA: -15,
    } as LiveSample;
    const group = liveChartGroups(false).find((group) => group.title === 'Load')!;
    expect(group.fields).toEqual(['dutyCycle', 'motorLoad', 'batteryLoad']);
    expect(
      group.fields.map((field) => liveChartReading(field, sample, 'metric', null, 0, currentLimits).value)
    ).toEqual([40, 60, 75]);
    expect(group.fields.map(liveChartColor)).toEqual(['var(--app-duty)', 'var(--app-danger)', 'var(--app-success)']);
    const geometry = liveChartGeometry([{ time: 0, sample, currentLimits }], group.fields, 'metric', 0);
    expect(geometry.low).toBe(0);
    expect(geometry.high).toBe(100);
    expect(geometry.paths).toEqual(['M1000.00,60.00', 'M1000.00,40.00', 'M1000.00,25.00']);
    expect(liveChartReading('motorLoad', sample, 'metric').value).toBeNull();
    expect(liveChartReading('batteryLoad', null, 'metric', null, 0, currentLimits).value).toBeNull();
  });
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
  it('adds the two live-view BMS readings to the temperature chart when a BMS is present', () => {
    const observedAtMs = Date.parse('2026-10-13T00:00:00.000Z');
    const bms = {
      status: 'attached',
      connection: { kind: 'can', canId: 42 },
      candidates: [],
      latestSnapshot: {
        observedAt: new Date(observedAtMs).toISOString(),
        totalVoltageV: 75,
        chargeVoltageV: 84,
        inputCurrentA: 0,
        inputIcCurrentA: 0,
        sessionAmpHoursAh: 0,
        sessionWattHoursWh: 0,
        cellVoltagesV: [],
        balancing: [],
        temperaturesC: [],
        icTemperatureC: 37,
        humiditySensorTemperatureC: 28,
        humidityPercent: 40,
        highestCircuitryTemperatureC: 38,
        highestCellTemperatureC: 29,
        stateOfCharge: 0.8,
        stateOfHealth: 0.95,
        reportedCanId: 42,
        lifetimeChargeAmpHoursAh: 0,
        lifetimeChargeWattHoursWh: 0,
        lifetimeDischargeAmpHoursAh: 0,
        lifetimeDischargeWattHoursWh: 0,
        pressurePa: 101_325,
        dataVersion: 1,
        status: 'OK',
      },
    } satisfies BmsState;
    const temperature = liveChartGroups(true).find((group) => group.title === 'Temperature');
    expect(temperature?.fields).toEqual(['controllerTempC', 'motorTempC', 'bmsCircuitryTempC', 'bmsHighestCellTempC']);
    expect(liveChartGroups(false).find((group) => group.title === 'Temperature')?.fields).toEqual([
      'controllerTempC',
      'motorTempC',
    ]);
    expect(liveChartReading('bmsCircuitryTempC', null, 'metric', bms, observedAtMs)).toMatchObject({
      label: 'BMS',
      value: 38,
      unit: '°C',
    });
    expect(liveChartReading('bmsHighestCellTempC', null, 'imperial', bms, observedAtMs)).toMatchObject({
      label: 'Cells',
      value: 84.2,
      unit: '°F',
    });
    const geometry = liveChartGeometry(
      [{ time: 0, sample: {} as LiveSample, bmsTemperatures: { circuitryC: 38, cellsC: 29 } }],
      ['bmsCircuitryTempC', 'bmsHighestCellTempC'],
      'metric',
      0
    );
    expect(geometry.paths).toHaveLength(2);
    expect(geometry.paths.every((path) => path.startsWith('M1000.00,'))).toBe(true);
  });
});
