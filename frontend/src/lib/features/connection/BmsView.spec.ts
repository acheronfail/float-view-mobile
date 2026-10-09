import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { BmsSnapshot, BmsState } from '$lib/backend/contract';
import BmsView from './BmsView.svelte';

const snapshot: BmsSnapshot = {
  observedAt: '2020-01-01T00:00:00.000Z',
  totalVoltageV: 63.5,
  chargeVoltageV: 75.6,
  inputCurrentA: -3.25,
  inputIcCurrentA: -3.2,
  sessionAmpHoursAh: 1.2,
  sessionWattHoursWh: 73.4,
  cellVoltagesV: Array.from({ length: 50 }, (_, index) => 3.5 + index / 1000),
  balancing: Array.from({ length: 50 }, (_, index) => index === 49),
  temperaturesC: Array.from({ length: 50 }, (_, index) => 20 + index / 10),
  icTemperatureC: 30,
  humiditySensorTemperatureC: 25,
  humidityPercent: 55.5,
  highestCellTemperatureC: 31,
  stateOfCharge: 0.8,
  stateOfHealth: 0.95,
  reportedCanId: 42,
  lifetimeChargeAmpHoursAh: 100,
  lifetimeChargeWattHoursWh: 6_000,
  lifetimeDischargeAmpHoursAh: 90,
  lifetimeDischargeWattHoursWh: 5_400,
  pressurePa: 101_325,
  dataVersion: 2,
  status: 'balancing',
};

const attached: BmsState = {
  status: 'attached',
  connection: { kind: 'can', canId: 42 },
  candidates: [],
  latestSnapshot: snapshot,
};

describe('BMS view', () => {
  it('renders the maximum bounded arrays, optional tail, route, and imperial temperatures', () => {
    const { body } = render(BmsView, { props: { bms: attached, units: 'imperial' } });
    const text = body.replace(/\s+/g, ' ');

    expect(text).toContain('CAN 42');
    expect(text).toContain('Cell 50');
    expect(text).toContain('3.549 V · balancing');
    expect(text).toContain('Sensor 50: 76.8 °F');
    expect(text).toContain('Reported CAN ID');
    expect(text).toContain('balancing');
    expect(text).toContain('IC current');
    expect(text).toContain('-3.20 A');
  });

  it('sorts ambiguous routes and directs configuration without showing value placeholders', () => {
    const bms: BmsState = {
      status: 'ambiguous',
      connection: null,
      latestSnapshot: null,
      candidates: [
        { connection: { kind: 'can', canId: 42 }, hardwareName: 'VBMS 42', vescVersion: '6.00' },
        { connection: { kind: 'can', canId: 3 }, hardwareName: 'VBMS 3', vescVersion: '6.00' },
        { connection: { kind: 'direct' }, hardwareName: 'Local VBMS', vescVersion: '6.00' },
      ],
    };
    const { body } = render(BmsView, { props: { bms, units: 'metric' } });
    const text = body.replace(/\s+/g, ' ');

    expect(text).toContain('Set Preferred BMS CAN ID in Board configuration');
    expect(text.indexOf('Local VBMS')).toBeLessThan(text.indexOf('VBMS 3'));
    expect(text.indexOf('VBMS 3')).toBeLessThan(text.indexOf('VBMS 42'));
    expect(text).not.toContain('BMS values will appear');
  });

  it('retains stale values and their original freshness while unavailable', () => {
    const { body } = render(BmsView, {
      props: { bms: { ...attached, status: 'unavailable' }, units: 'metric' },
    });
    const text = body.replace(/\s+/g, ' ');

    expect(text).toContain('retained from their original observation time');
    expect(text).toContain('ago');
    expect(text).toContain('30.0 °C');
    expect(text).toContain('63.50 V');
  });
});
