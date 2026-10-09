import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import type { ActiveConnection, BmsSnapshot, LiveSample } from '$lib/backend/contract';
import { liveCharging } from './charging';
import { liveStatus } from './live-status';
import WidgetPower from './WidgetPower.svelte';

const now = Date.parse('2026-10-10T00:00:00Z');
const connection = {
  state: 'ready',
  latestSample: { charging: false, packageState: 'ready', packVoltageV: 78 } as LiveSample,
  bms: {
    status: 'attached',
    connection: { kind: 'can', canId: 10 },
    candidates: [],
    latestSnapshot: { observedAt: new Date(now).toISOString(), status: 'CHARGING', inputCurrentA: 0 } as BmsSnapshot,
  },
} satisfies Pick<ActiveConnection, 'state' | 'latestSample' | 'bms'>;

function withSnapshot(values: Partial<BmsSnapshot>) {
  return { ...connection, bms: { ...connection.bms, latestSnapshot: { ...connection.bms.latestSnapshot, ...values } } };
}

describe('live charging presentation', () => {
  it('shows fresh VBMS charging in the header and battery despite zero Refloat and input current', () => {
    const charging = liveCharging(connection, now);
    expect(charging).toBe(true);
    expect(liveStatus('ready', connection.latestSample, charging).display).toBe('Charging');
    const { body } = render(WidgetPower, { props: { sample: connection.latestSample, estimate: null, charging } });
    expect(body).toContain('Charging');
    expect(body).toContain('battery-charging-pulse');
  });
  it('expires unchanged snapshots and rejects invalid or future timestamps', () => {
    expect(liveCharging(connection, now + 4999)).toBe(true);
    expect(liveCharging(connection, now + 5000)).toBe(false);
    expect(liveCharging(connection, now - 1)).toBe(false);
    expect(liveCharging(withSnapshot({ observedAt: 'invalid' }), now)).toBe(false);
  });
  it('rejects unavailable, ambiguous, reconnecting and missing data', () => {
    for (const status of ['unavailable', 'discovering', 'ambiguous', 'not_found'] as const) {
      expect(liveCharging({ ...connection, bms: { ...connection.bms, status } }, now)).toBe(false);
    }
    expect(liveCharging({ ...connection, state: 'reconnecting' }, now)).toBe(false);
    expect(liveCharging({ ...connection, bms: { ...connection.bms, latestSnapshot: null } }, now)).toBe(false);
    expect(liveCharging({ ...connection, bms: { ...connection.bms, connection: null } }, now)).toBe(false);
    expect(liveCharging(null, now)).toBe(false);
  });
  it('requires explicit BMS charging and preserves independent Refloat charging', () => {
    for (const status of [null, 'IDLE', 'DISCHARGING', 'NOT CHARGING']) {
      expect(liveCharging(withSnapshot({ status }), now)).toBe(false);
    }
    expect(
      liveCharging({ ...connection, latestSample: { ...connection.latestSample, charging: true } }, now + 6000)
    ).toBe(true);
    expect(liveCharging({ ...connection, latestSample: null }, now)).toBe(true);
    expect(liveStatus('ready', null, true).display).toBe('Charging');
  });
  it('prioritizes faults and connection state', () => {
    expect(liveStatus('ready', { ...connection.latestSample, firmwareFault: 'underVoltage' }, true).display).toContain(
      'Firmware fault'
    );
    expect(liveStatus('reconnecting', connection.latestSample, true).display).toBe('reconnecting');
  });
});
