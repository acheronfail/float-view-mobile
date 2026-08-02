import { describe, expect, it } from 'vitest';
import { MockBackend } from '../../../backend-mock/api-v1';
import type { AppState } from './contract';

describe('deterministic mock backend', () => {
  it('implements state and publishes full monotonic state changes', () => {
    const events: unknown[] = [];
    const backend = new MockBackend((event) => events.push(event));
    expect(backend.handle({ id: 1, method: 'state.get', params: {} })).toMatchObject({
      id: 1,
      result: { revision: 0 },
    });
    expect(backend.handle({ id: 2, method: 'bluetooth.scan', params: {} })).toEqual({
      id: 2,
      result: { accepted: true },
    });
    expect(events).toEqual([
      expect.objectContaining({
        event: 'state.changed',
        data: expect.objectContaining({ revision: 1 }),
      }),
    ]);
  });

  it('rejects malformed, unknown, and stale mutations with stable errors', () => {
    const backend = new MockBackend();
    expect(backend.handlePayload('{')).toMatchObject({
      error: { code: 'invalid_request' },
    });
    expect(backend.handle({ id: 1, method: 'other', params: {} })).toMatchObject({
      error: { code: 'invalid_request' },
    });
    expect(
      backend.handle({
        id: 2,
        method: 'settings.update',
        params: { expectedRevision: 9, patch: {} },
      })
    ).toMatchObject({ error: { code: 'conflict' } });
  });

  it('rejects control payloads that exceed 65,536 UTF-8 bytes', () => {
    const backend = new MockBackend();
    expect(backend.handlePayload('😀'.repeat(16_385))).toMatchObject({
      error: { code: 'invalid_request', message: 'Message is too large' },
    });
  });

  it('covers every mandatory method', () => {
    const backend = new MockBackend();
    const request = (id: number, method: string, params: Record<string, unknown> = {}) =>
      backend.handle({ id, method, params });
    request(1, 'bluetooth.scan');
    request(2, 'device.connect', { deviceId: 'mock-board' });
    expect(request(3, 'device.disconnect')).toMatchObject({ result: { accepted: true } });
    expect(request(4, 'boardProfile.get', { savedDeviceId: 'saved-board' })).toHaveProperty('result');
    expect(request(5, 'session.get', { sessionId: 'session-1' })).toHaveProperty('result');
    expect(
      request(6, 'session.prune.preview', { scope: 'one', sessionId: 'session-1', targetResolutionSeconds: 10 })
    ).toHaveProperty('result');
    const prune = request(7, 'session.prune.start', {
      scope: 'one',
      sessionId: 'session-1',
      targetResolutionSeconds: 10,
    }) as { result: { operationId: string } };
    expect(prune).toHaveProperty('result');
    expect(request(8, 'session.prune.cancel', { operationId: prune.result.operationId })).toHaveProperty('result');
    expect(request(9, 'session.export', { sessionId: 'session-1', format: 'csv' })).toHaveProperty('result');
    expect(
      request(10, 'savedDevice.update', {
        savedDeviceId: 'saved-board',
        expectedRevision: 0,
        customDisplayName: 'My board',
      })
    ).toMatchObject({ result: { displayName: 'My board', revision: 1 } });
  });

  it('publishes complete live snapshots at the configured persistence cadence', () => {
    const events: {
      data: { activeConnection: { latestSample: { packageState: string } } };
    }[] = [];
    const backend = new MockBackend((event) => events.push(event as (typeof events)[number]));
    backend.handle({ id: 1, method: 'bluetooth.scan', params: {} });
    backend.handle({ id: 2, method: 'device.connect', params: { deviceId: 'mock-board' } });
    backend.publishTelemetry();
    const liveSample = events.at(-1)?.data.activeConnection.latestSample;
    backend.simulateConnectionState('reconnecting');
    expect(backend.telemetryCadenceMs).toBe(100);
    expect(liveSample).toMatchObject({
      rideState: 'riding',
      packageState: 'running',
      footpadState: 'both',
    });
    expect(events.at(-1)?.data.activeConnection).toMatchObject({
      state: 'reconnecting',
      latestSample: liveSample,
    });
  });

  it('sorts bounded controller choices and persists one supported CAN selection', () => {
    const backend = new MockBackend();
    backend.handle({ id: 1, method: 'bluetooth.scan', params: {} });
    backend.handle({ id: 2, method: 'device.connect', params: { deviceId: 'mock-board' } });
    backend.simulateControllerSelection([
      {
        canId: 42,
        hardwareName: 'THOR300',
        firmwareName: 'Refloat',
        vescVersion: '6.06',
        vescSupported: true,
      },
      {
        canId: 7,
        hardwareName: 'Other VESC',
        firmwareName: null,
        vescVersion: '5.03',
        vescSupported: false,
      },
    ]);
    expect(
      (
        backend.handle({ id: 3, method: 'state.get', params: {} }) as { result: AppState }
      ).result.activeConnection?.controllerCandidates.map((candidate) => candidate.canId)
    ).toEqual([7, 42]);
    expect(
      backend.handle({
        id: 4,
        method: 'device.selectController',
        params: { deviceId: 'mock-board', canId: 7 },
      })
    ).toMatchObject({ error: { code: 'invalid_state' } });
    expect(
      backend.handle({
        id: 5,
        method: 'device.selectController',
        params: { deviceId: 'mock-board', canId: 42 },
      })
    ).toMatchObject({ result: { accepted: true } });
    expect(
      backend.handle({
        id: 6,
        method: 'device.selectController',
        params: { deviceId: 'mock-board', canId: 42 },
      })
    ).toMatchObject({ error: { code: 'invalid_state' } });
    expect(
      backend.handle({
        id: 7,
        method: 'device.selectController',
        params: { deviceId: 'mock-board', canId: 254 },
      })
    ).toMatchObject({ error: { code: 'invalid_request' } });
  });

  it('never publishes a partial controller prompt beyond the contract bound', () => {
    const backend = new MockBackend();
    backend.handle({ id: 1, method: 'bluetooth.scan', params: {} });
    backend.handle({ id: 2, method: 'device.connect', params: { deviceId: 'mock-board' } });
    backend.simulateControllerSelection(
      Array.from({ length: 17 }, (_, canId) => ({
        canId,
        hardwareName: `Controller ${canId}`,
        firmwareName: 'Refloat',
        vescVersion: '6.06',
        vescSupported: true,
      }))
    );

    expect(
      (backend.handle({ id: 3, method: 'state.get', params: {} }) as { result: AppState }).result.activeConnection
    ).toMatchObject({
      state: 'ready',
      controllerCandidates: [],
    });
  });
});
