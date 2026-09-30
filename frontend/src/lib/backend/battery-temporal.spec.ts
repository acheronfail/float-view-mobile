import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { MockBattery } from '../../../backend-mock/battery';
import { MockBackend } from '../../../backend-mock/api-v1';
import type { ActiveConnection, AppState, BoardProfileValues } from './contract';
const cases = JSON.parse(
  readFileSync(new URL('../../../../conformance/fixtures/battery-temporal.json', import.meta.url), 'utf8')
) as {
  name: string;
  capacityAh: number | null;
  minimumV: number;
  expectedLevel: number;
  samples: { timeMs: number; voltage: number; current: number; charging?: boolean }[];
}[];
function active(): ActiveConnection {
  const backend = new MockBackend();
  backend.handle({ id: 1, method: 'bluetooth.scan', params: {} });
  backend.handle({ id: 2, method: 'device.connect', params: { deviceId: 'mock-board', trackingMode: 'saved' } });
  const state = (backend.handle({ id: 3, method: 'state.get', params: {} }) as { result: AppState }).result;
  return { ...state.activeConnection!, state: 'ready', currentLimits: null };
}
const values: BoardProfileValues = {
  seriesCount: 20,
  parallelCount: 2,
  batteryCellProfileId: 'generic-linear',
  batteryMinimumCellVoltageV: 2.7,
  batteryCapacityAh: 9,
};
describe('temporal battery mock independent oracles', () => {
  for (const test of cases)
    it(test.name, () => {
      const mock = new MockBattery();
      const a = active();
      let result;
      for (const [i, s] of test.samples.entries()) {
        a.latestSample = {
          ...a.latestSample!,
          sequence: i + 1,
          elapsedMs: s.timeMs,
          packVoltageV: s.voltage,
          batteryCurrentA: s.current,
          charging: s.charging ?? false,
        };
        result = mock.update(a, {
          revision: 1,
          values: { ...values, batteryCapacityAh: test.capacityAh, batteryMinimumCellVoltageV: test.minimumV },
        });
      }
      expect(result?.level).toBeCloseTo(test.expectedLevel, 12);
      expect(result?.provisional).toBe(true);
    });
  it('does not advance repeated/reordered samples or identity-only edits and resumes without a gap', () => {
    const mock = new MockBattery(),
      a = active();
    a.latestSample = { ...a.latestSample!, sequence: 1, elapsedMs: 0, packVoltageV: 78, batteryCurrentA: 60 };
    const first = mock.update(a, { revision: 1, values });
    a.latestSample = { ...a.latestSample, sequence: 2, elapsedMs: 1000, packVoltageV: 63 };
    const integrated = mock.update(a, { revision: 1, values });
    expect(integrated.method).toBe('charge_integrated');
    expect(mock.update(a, { revision: 2, values: { ...values, batteryPresetId: 'identity-only' } }).level).toBe(
      integrated.level
    );
    a.latestSample = { ...a.latestSample, sequence: 1, elapsedMs: 500 };
    expect(mock.update(a, { revision: 2, values }).level).toBe(integrated.level);
    mock.update({ ...a, state: 'reconnecting' }, { revision: 2, values });
    a.latestSample = { ...a.latestSample, sequence: 1, elapsedMs: 15000, packVoltageV: 84 };
    expect(mock.update(a, { revision: 2, values }).level).toBe(integrated.level);
    expect(mock.update(a, { revision: 3, values: { ...values, batteryCapacityAh: null } }).level).toBe(1);
    expect(first.level).toBeCloseTo(0.8, 12);
  });
  it('qualifies settled rest, stops at renewed load and breaks invalid current/voltage pairs', () => {
    const mock = new MockBattery(),
      a = active();
    let e;
    for (let i = 0; i <= 30; i++) {
      a.latestSample = {
        ...a.latestSample!,
        sequence: i + 1,
        elapsedMs: i * 1000,
        packVoltageV: 78,
        batteryCurrentA: 0,
        charging: false,
      };
      e = mock.update(a, { revision: 1, values });
      expect(e.provisional).toBe(i < 30);
    }
    a.latestSample = { ...a.latestSample!, sequence: 32, elapsedMs: 31000, packVoltageV: null };
    expect(mock.update(a, { revision: 1, values })).toMatchObject({ level: null, method: null, provisional: false });
    a.latestSample = { ...a.latestSample, sequence: 33, elapsedMs: 32000, packVoltageV: 63, batteryCurrentA: 60 };
    expect(mock.update(a, { revision: 1, values })).toMatchObject({
      level: e!.level,
      method: 'voltage_smoothed',
      provisional: true,
    });
    a.latestSample = { ...a.latestSample, sequence: 34, elapsedMs: 33000, batteryCurrentA: null };
    expect(mock.update(a, { revision: 1, values }).method).toBe('voltage_smoothed');
  });
});
