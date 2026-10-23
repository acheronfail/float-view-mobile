import { describe, it, expect } from 'vitest';
import type { ActiveConnection } from '$lib/backend/contract';
import { LiveHistoryBuffer } from './live-history';
import { liveChartGeometry } from './live-charts';
const wall = Date.parse('2026-10-23T00:00:00Z');
const connection = (sequence = 1) =>
  ({
    deviceId: 'a',
    savedDeviceId: 'board',
    trackingMode: 'saved',
    state: 'ready',
    latestSample: { sequence, observedAt: new Date(wall).toISOString(), speedKph: 10 },
    bms: { status: 'not_found' },
    batteryEstimate: { level: 0.5 },
  }) as ActiveConnection;
describe('shared live history admission', () => {
  it('preserves captured BMS freshness and raw reading semantics across presentations', () => {
    const buffer = new LiveHistoryBuffer();
    const active = connection();
    active.latestSample = {
      ...active.latestSample!,
      observedAt: new Date(wall - 1000).toISOString(),
      dutyCycle: -0.4,
      load: { duty: 0.4, motor: null, battery: null, winner: { source: 'duty', fraction: 0.4 } },
    };
    active.bms = {
      status: 'attached',
      connection: { kind: 'direct' },
      candidates: [],
      latestSnapshot: {
        observedAt: new Date(wall - 5).toISOString(),
        stateOfCharge: 0.65,
        cellVoltagesV: [],
        totalVoltageV: 75,
        chargeVoltageV: 84,
        inputCurrentA: 0,
        inputIcCurrentA: 0,
        sessionAmpHoursAh: 0,
        sessionWattHoursWh: 0,
        balancing: [],
        temperaturesC: [],
        icTemperatureC: 25,
        humiditySensorTemperatureC: 25,
        humidityPercent: 40,
        highestCircuitryTemperatureC: 25,
        highestCellTemperatureC: 25,
        stateOfHealth: 1,
        reportedCanId: 42,
        lifetimeChargeAmpHoursAh: 0,
        lifetimeChargeWattHoursWh: 0,
        lifetimeDischargeAmpHoursAh: 0,
        lifetimeDischargeWattHoursWh: 0,
        pressurePa: 101325,
        dataVersion: 1,
        status: 'OK',
      },
    };
    buffer.accept(active, wall, 100);
    const bms = liveChartGeometry(buffer.points, ['bmsStateOfCharge'], 'metric', 100, { rawReadings: true });
    expect(bms.paths[0]).toMatch(/^M1000/);
    const raw = liveChartGeometry(buffer.points, ['dutyCycle'], 'metric', 100, { rawReadings: true });
    const load = liveChartGeometry(buffer.points, ['dutyCycle'], 'metric', 100);
    expect(raw.low).toBeLessThan(-40);
    expect(load.low).toBe(0);
    expect(load.high).toBe(100);
  });
  it('deduplicates, prunes and captures estimated values without reinterpreting points', () => {
    const buffer = new LiveHistoryBuffer();
    buffer.accept(connection(), wall, 0);
    buffer.accept(connection(), wall, 100);
    expect(buffer.points).toHaveLength(1);
    expect(buffer.points[0].estimatedLevel).toBe(0.5);
    buffer.accept(connection(2), wall, 200);
    expect(buffer.points).toHaveLength(2);
    buffer.prune(30201);
    expect(buffer.points).toHaveLength(0);
  });
  it('clears on ready boundaries, identity/sequence changes and frontend transport loss', () => {
    const buffer = new LiveHistoryBuffer();
    buffer.accept(connection(5), wall, 0);
    buffer.accept(connection(1), wall, 1);
    expect(buffer.points).toHaveLength(1);
    buffer.accept({ ...connection(2), savedDeviceId: 'other' }, wall, 2);
    expect(buffer.points).toHaveLength(1);
    buffer.accept({ ...connection(2), state: 'reconnecting' }, wall, 3);
    expect(buffer.points).toHaveLength(0);
    buffer.accept(connection(2), wall, 4);
    buffer.accept(connection(3), wall, 5, false);
    expect(buffer.points).toHaveLength(0);
    buffer.accept(connection(3), wall + 2000, 6);
    expect(buffer.points).toHaveLength(0);
  });
  it('bounds all chart consumers to one 1500-point collection', () => {
    const buffer = new LiveHistoryBuffer();
    for (let i = 0; i < 2000; i++) buffer.accept(connection(i), wall, i);
    expect(buffer.points).toHaveLength(1500);
    const chartA = buffer.points,
      chartB = buffer.points;
    expect(chartA).toBe(chartB);
  });
});
