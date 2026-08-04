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
    expect(request(11, 'session.list', { limit: 1 })).toMatchObject({
      result: { items: expect.any(Array), catalogRevision: expect.any(Number) },
    });
    expect(request(12, 'sessionGroup.unlink', { sessionGroupId: 'group-1' })).toMatchObject({
      error: { code: 'invalid_request' },
    });
  });

  it('pages the standalone catalog beyond fifty sessions without offset state', () => {
    const backend = new MockBackend();
    for (let index = 0; index < 51; index += 1) {
      backend.handle({ id: index * 3 + 1, method: 'bluetooth.scan', params: {} });
      backend.handle({ id: index * 3 + 2, method: 'device.connect', params: { deviceId: 'mock-board' } });
      backend.handle({ id: index * 3 + 3, method: 'device.disconnect', params: {} });
    }
    const items: string[] = [];
    const revisions = new Set<number>();
    let cursor: string | null = null;
    do {
      const response = backend.handle({
        id: items.length + 100,
        method: 'session.list',
        params: { limit: 25, cursor },
      }) as {
        result: { items: Array<{ sessionId: string }>; catalogRevision: number; nextCursor: string | null };
      };
      expect(response.result.items.length).toBeLessThanOrEqual(25);
      items.push(...response.result.items.map((item) => item.sessionId));
      revisions.add(response.result.catalogRevision);
      cursor = response.result.nextCursor;
    } while (cursor);
    expect(items.length).toBeGreaterThan(50);
    expect(new Set(items).size).toBe(items.length);
    expect(revisions.size).toBe(1);
  });

  it('keeps telemetry-only state changes out of catalog revision invalidation', () => {
    const backend = new MockBackend();
    backend.handle({ id: 1, method: 'bluetooth.scan', params: {} });
    backend.handle({ id: 2, method: 'device.connect', params: { deviceId: 'mock-board' } });
    const before = (backend.handle({ id: 3, method: 'state.get', params: {} }) as { result: AppState }).result;
    backend.publishTelemetry();
    const after = (backend.handle({ id: 4, method: 'state.get', params: {} }) as { result: AppState }).result;
    expect(after.revision).toBeGreaterThan(before.revision);
    expect(after.sessionCatalogRevision).toBe(before.sessionCatalogRevision);
    expect(after).not.toHaveProperty('recentSessions');
    expect(after).not.toHaveProperty('sessionGroups');
  });

  it('rejects a catalog cursor after a visible mutation', () => {
    const backend = new MockBackend();
    const first = backend.handle({ id: 1, method: 'session.list', params: { limit: 1 } }) as {
      result: { nextCursor: string | null };
    };
    expect(first.result.nextCursor).not.toBeNull();
    backend.handle({ id: 2, method: 'session.delete', params: { sessionId: 'session-1' } });
    expect(
      backend.handle({ id: 3, method: 'session.list', params: { limit: 1, cursor: first.result.nextCursor } })
    ).toMatchObject({ error: { code: 'conflict' } });
  });

  it('publishes complete live snapshots at the configured persistence cadence', () => {
    const events: {
      data: {
        activeConnection: {
          latestSample: { packageState: string };
          latestLocation: {
            latitude: number;
            longitude: number;
            accuracyM: number | null;
            observedAt: string;
          };
        };
      };
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
    const liveLocation = events.at(-1)?.data.activeConnection.latestLocation;
    expect(liveLocation).toEqual({
      latitude: -33.8688,
      longitude: 151.2093,
      accuracyM: 12,
      observedAt: expect.any(String),
    });
    expect(events.at(-1)?.data.activeConnection).toMatchObject({
      state: 'reconnecting',
      latestSample: liveSample,
      latestLocation: liveLocation,
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
