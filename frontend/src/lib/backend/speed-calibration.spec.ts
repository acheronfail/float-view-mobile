import { describe, expect, it, vi } from 'vitest';
import { MockBackend } from '../../../backend-mock/api-v1';
import type { AppState } from './contract';
function fixture() {
  const b = new MockBackend(undefined, { safetyDisclaimerAccepted: true });
  const request = (method: string, params: object = {}) => b.handle({ id: 1, method, params });
  const state = () => (request('state.get') as { result: AppState }).result;
  request('bluetooth.scan');
  request('device.connect', { deviceId: 'mock-board', trackingMode: 'saved' });
  b.publishTelemetry();
  b.simulatePermissions(
    state().permissions.map((p) =>
      p.permission === 'location' ? { ...p, status: 'granted', scope: 'always', accuracy: 'precise', action: null } : p
    )
  );
  return { b, request, state };
}
describe('calibration contract', () => {
  it('preserves route settings and completes automatically and gates application on stopping', () => {
    const { b, request, state } = fixture();
    expect(request('speedCalibration.start', { diameterMm: 280 })).toMatchObject({ result: { accepted: true } });
    expect(state().settings.locationRecordingEnabled).toBe(false);
    b.simulateRideState('riding');
    b.simulateSpeedCalibrationResult(268.8);
    expect(state().speedCalibration.status).toBe('ready');
    expect(request('speedCalibration.apply')).toMatchObject({ error: { code: 'invalid_state' } });
    b.simulateRideState('idle');
    expect(request('speedCalibration.apply')).toMatchObject({ result: { accepted: true } });
    expect(state().speedCalibration).toMatchObject({ status: 'applied', currentDiameterMm: 268.8 });
  });
  it('retains an advisory result for unsupported layouts and cancels across reconnect', () => {
    const { b, request, state } = fixture();
    request('speedCalibration.start', { diameterMm: 280 });
    b.simulateSpeedCalibrationResult(269, false);
    expect(state().speedCalibration).toMatchObject({
      status: 'ready',
      recommendedDiameterMm: 269,
      applyAvailable: false,
      applyBlockReason: 'unsupportedSetup',
    });
    expect(request('speedCalibration.apply')).toMatchObject({ error: { code: 'invalid_state' } });
    b.simulateConnectionState('reconnecting');
    expect(state().speedCalibration.status).toBe('idle');
  });
});

it('keeps one absolute three-minute deadline through state reads', () => {
  vi.useFakeTimers();
  try {
    vi.setSystemTime(new Date('2026-11-14T02:00:00Z'));
    const { b, request, state } = fixture();
    request('speedCalibration.start', { diameterMm: 280 });
    expect(state().speedCalibration.status).toBe('collecting');
    vi.advanceTimersByTime(90_000);
    b.publishTelemetry();
    expect(state().speedCalibration.status).toBe('collecting');
    request('state.get');
    vi.advanceTimersByTime(90_000);
    b.publishTelemetry();
    expect(state().speedCalibration).toMatchObject({
      status: 'insufficient',
      recommendedDiameterMm: null,
    });
    expect(request('diagnostics.list')).toMatchObject({ result: { incidents: [{ reason: 'calibration_failure' }] } });
  } finally {
    vi.useRealTimers();
  }
});
