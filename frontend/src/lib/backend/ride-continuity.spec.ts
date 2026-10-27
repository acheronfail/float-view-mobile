import { afterEach, expect, it, vi } from 'vitest';
import { MockBackend } from '../../../backend-mock/api-v1';
import type { AppState } from './contract';
const state = (b: MockBackend) => (b.handle({ id: 1, method: 'state.get', params: {} }) as { result: AppState }).result;
const connect = (b: MockBackend) => {
  b.handle({ id: 2, method: 'bluetooth.scan', params: {} });
  b.handle({ id: 3, method: 'device.connect', params: { deviceId: 'mock-board', trackingMode: 'saved' } });
};
afterEach(() => vi.useRealTimers());
it('resumes repeated breaks and rejects stale completion without affecting the ride', () => {
  vi.useFakeTimers();
  const b = new MockBackend();
  connect(b);
  b.publishTelemetry();
  b.publishTelemetry();
  const before = state(b).activeConnection!.latestSample!;
  const id = state(b).activeConnection!.activeSession!.sessionId;
  b.handle({ id: 4, method: 'device.disconnect', params: {} });
  const deadline = state(b).pausedRide!.resumeUntil;
  vi.advanceTimersByTime(30 * 60_000);
  connect(b);
  expect(state(b).activeConnection!.activeSession!.sessionId).toBe(id);
  b.publishTelemetry();
  const after = state(b).activeConnection!.latestSample!;
  expect(after.sequence).toBeGreaterThan(before.sequence);
  expect(after.elapsedMs).toBeGreaterThan(before.elapsedMs + 29 * 60_000);
  expect(after.ridingElapsedMs).toBe(before.ridingElapsedMs);
  expect(before.distanceM).toBeGreaterThan(0);
  expect(after.distanceM).toBe(before.distanceM);
  expect(after.maxSpeedKph).toBe(before.maxSpeedKph);
  expect(after.maxSpeedKph).toBe(24.3);
  b.publishTelemetry();
  expect(state(b).activeConnection!.latestSample!.distanceM).toBeGreaterThan(after.distanceM!);
  expect(b.handle({ id: 5, method: 'session.end', params: { sessionId: 'stale' } })).toMatchObject({
    error: { code: 'invalid_state' },
  });
  b.handle({ id: 6, method: 'device.disconnect', params: {} });
  expect(Date.parse(state(b).pausedRide!.resumeUntil)).toBeGreaterThan(Date.parse(deadline));
  b.handle({ id: 7, method: 'session.end', params: { sessionId: id } });
  expect(state(b).pausedRide).toBeNull();
  connect(b);
  expect(state(b).activeConnection!.activeSession!.sessionId).not.toBe(id);
  b.publishTelemetry();
  expect(state(b).activeConnection!.latestSample!.distanceM).toBe(0);
});
it('expires the paused ride and accepts only whole minute settings', () => {
  vi.useFakeTimers();
  const b = new MockBackend();
  for (const value of [0, 1441, 1.5])
    expect(
      b.handle({
        id: 2,
        method: 'settings.update',
        params: { expectedRevision: 0, patch: { rideResumeWindowMinutes: value } },
      })
    ).toMatchObject({ error: { code: 'invalid_request' } });
  connect(b);
  const id = state(b).activeConnection!.activeSession!.sessionId;
  b.handle({ id: 4, method: 'device.disconnect', params: {} });
  vi.advanceTimersByTime(60 * 60_000);
  expect(state(b).pausedRide).toBeNull();
  connect(b);
  expect(state(b).activeConnection!.activeSession!.sessionId).not.toBe(id);
});

it('extends the paused ride repeatedly, replaces expiry and never shortens its window', () => {
  vi.useFakeTimers();
  const b = new MockBackend();
  connect(b);
  const id = state(b).activeConnection!.activeSession!.sessionId;
  const extend = (sessionId = id) => b.handle({ id: 10, method: 'session.extendPause', params: { sessionId } });
  expect(extend()).toMatchObject({ error: { code: 'invalid_state' } });
  b.handle({ id: 4, method: 'device.disconnect', params: {} });
  const original = Date.parse(state(b).pausedRide!.resumeUntil);
  vi.advanceTimersByTime(30 * 60_000);
  expect(extend('stale')).toMatchObject({ error: { code: 'invalid_state' } });
  const revision = state(b).revision;
  expect(extend()).toMatchObject({ result: { accepted: true } });
  expect(state(b).revision).toBeGreaterThan(revision);
  expect(Date.parse(state(b).pausedRide!.resumeUntil)).toBe(original + 30 * 60_000);
  expect(state(b).activeConnection).toBeNull();
  vi.advanceTimersByTime(30 * 60_000);
  expect(state(b).pausedRide!.sessionId).toBe(id);
  expect(extend()).toMatchObject({ result: { accepted: true } });
  const renewed = state(b).pausedRide!.resumeUntil;
  b.handle({
    id: 11,
    method: 'settings.update',
    params: { expectedRevision: state(b).settings.revision, patch: { rideResumeWindowMinutes: 1 } },
  });
  expect(extend()).toMatchObject({ result: { accepted: true } });
  expect(state(b).pausedRide!.resumeUntil).toBe(renewed);
  vi.advanceTimersByTime(60 * 60_000);
  expect(state(b).pausedRide).toBeNull();
  expect(extend()).toMatchObject({ error: { code: 'invalid_state' } });
});
it('rejects malformed extension and resumes the same ride after its original deadline', () => {
  vi.useFakeTimers();
  const b = new MockBackend();
  connect(b);
  const id = state(b).activeConnection!.activeSession!.sessionId;
  b.handle({ id: 4, method: 'device.disconnect', params: {} });
  for (const params of [{}, { sessionId: 1 }, { sessionId: '' }, { sessionId: id, extra: true }])
    expect(b.handle({ id: 5, method: 'session.extendPause', params })).toMatchObject({
      error: { code: 'invalid_request' },
    });
  vi.advanceTimersByTime(50 * 60_000);
  b.handle({ id: 6, method: 'session.extendPause', params: { sessionId: id } });
  vi.advanceTimersByTime(20 * 60_000);
  connect(b);
  expect(state(b).activeConnection!.activeSession!.sessionId).toBe(id);
  expect(state(b).pausedRide).toBeNull();
  vi.advanceTimersByTime(60 * 60_000);
  expect(state(b).activeConnection!.activeSession!.sessionId).toBe(id);
});
