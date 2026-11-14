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
    expect(state().speedCalibration.status).toBe('refining');
    expect(request('speedCalibration.apply')).toMatchObject({ error: { code: 'invalid_state' } });
    request('speedCalibration.finish');
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
    request('speedCalibration.finish');
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

it('publishes explicit progress, preserves it through pauses and completes only with a result', () => {
  const { b, request, state } = fixture();
  request('speedCalibration.start', { diameterMm: 280 });
  expect(state().speedCalibration.progressPercent).toBe(0);
  b.simulateSpeedCalibrationProgress('needsMoreMeasurements', 42, 82);
  expect(state().speedCalibration).toMatchObject({ status: 'collecting', progressPercent: 82 });
  b.simulateSpeedCalibrationProgress('gpsUncertain', 42, 82);
  b.simulateSpeedCalibrationProgress('collecting', 43, 100);
  expect(state().speedCalibration).toMatchObject({ status: 'collecting', progressPercent: 82 });
  b.simulateSpeedCalibrationResult(268.8);
  expect(state().speedCalibration).toMatchObject({ status: 'refining', progressPercent: 100 });
  request('speedCalibration.finish');
  expect(state().speedCalibration).toMatchObject({ status: 'ready', progressPercent: 100 });
  request('speedCalibration.cancel');
  request('speedCalibration.start', { diameterMm: 280 });
  expect(state().speedCalibration.progressPercent).toBe(0);
});

it('keeps an approved result through later uncertainty and finalizes it at the original deadline', () => {
  vi.useFakeTimers();
  try {
    const { b, request, state } = fixture();
    request('speedCalibration.start', { diameterMm: 280 });
    expect(request('speedCalibration.finish')).toMatchObject({ error: { code: 'invalid_state' } });
    vi.advanceTimersByTime(60_000);
    b.simulateSpeedCalibrationResult(268.8);
    b.simulateSpeedCalibrationProgress('gpsUncertain', 35, 100);
    expect(state().speedCalibration).toMatchObject({
      status: 'refining',
      recommendedDiameterMm: 268.8,
      applyAvailable: false,
    });
    vi.advanceTimersByTime(120_000);
    b.publishTelemetry();
    expect(state().speedCalibration).toMatchObject({ status: 'ready', recommendedDiameterMm: 268.8 });
    expect(request('diagnostics.list')).toMatchObject({
      result: { incidents: expect.arrayContaining([expect.objectContaining({ reason: 'calibration_complete' })]) },
    });
    expect(request('speedCalibration.finish')).toMatchObject({ result: { accepted: true } });
  } finally {
    vi.useRealTimers();
  }
});
