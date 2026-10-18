import { describe, expect, it } from 'vitest';
import { validateConnectionContextEvent, validateSessionDataRequest, validateSessionDataResult } from './session-data';
import { MockBackend } from '../../../backend-mock/api-v1';
import type { AppState, ConnectionContextEvent } from './contract';
const firmware = { major: 6, minor: 6, testVersion: 1, hardwareName: 'Thor300', firmwareName: null };
const event: ConnectionContextEvent = {
  sequence: 1,
  elapsedMs: 0,
  observedAtMs: 1000,
  context: {
    version: 1,
    refloatVersion: '1.3.0',
    controller: firmware,
    transport: { ...firmware, firmwareName: 'VESC Express' },
    connection: { kind: 'can', canId: 42 },
  },
};
const request = {
  kind: 'connectionContexts' as const,
  sessionId: 's',
  snapshot: {
    historyRevision: 0,
    committedSampleSequence: 0,
    committedEventSequence: 2,
    committedElapsedMs: 1000,
    committedLocationSequence: 0,
    committedLocationElapsedMs: 0,
  },
  range: { startElapsedMs: 0, endElapsedMs: 1000 },
  limit: 64,
};
describe('recorded connection context', () => {
  it('validates closed public facts, legacy versions and numeric bounds', () => {
    expect(() => validateConnectionContextEvent(event)).not.toThrow();
    expect(() =>
      validateConnectionContextEvent({ ...event, context: { ...event.context, refloatVersion: '0.9' } })
    ).not.toThrow();
    for (const context of [
      { ...event.context, version: 2 },
      { ...event.context, connection: { kind: 'can', canId: 254 } },
      { ...event.context, controller: { ...firmware, controllerUuid: 'private' } },
      { ...event.context, transport: { ...firmware, hardwareName: 'é'.repeat(65) } },
    ])
      expect(() => validateConnectionContextEvent({ ...event, context })).toThrow();
    expect(() => validateConnectionContextEvent({ ...event, sequence: Number.MAX_SAFE_INTEGER + 1 })).toThrow();
  });
  it('bounds pages and excludes stale or out-of-order events', () => {
    expect(validateSessionDataRequest(request)).toEqual(request);
    expect(() => validateSessionDataRequest({ ...request, limit: 65 })).toThrow();
    const result = { ...request, contexts: [event], nextCursor: null };
    expect(validateSessionDataResult(result, request)).toEqual(result);
    for (const contexts of [[event, event], [{ ...event, sequence: 3 }], [{ ...event, elapsedMs: 1001 }]])
      expect(() => validateSessionDataResult({ ...result, contexts }, request)).toThrow();
    expect(() => validateSessionDataResult({ ...result, contexts: [], nextCursor: 'cursor' }, request)).toThrow();
  });
  it('fits a maximum page of escaped firmware names within the encoded response bound', () => {
    const largeFirmware = {
      major: 255,
      minor: 255,
      testVersion: 255,
      hardwareName: '\0'.repeat(128),
      firmwareName: '\0'.repeat(128),
    };
    const contexts = Array.from({ length: 64 }, (_, i) => ({
      ...event,
      sequence: i + 1,
      context: { ...event.context, refloatVersion: '255.255.255', controller: largeFirmware, transport: largeFirmware },
    }));
    const largeRequest = { ...request, snapshot: { ...request.snapshot, committedEventSequence: 64 } };
    const result = { ...largeRequest, contexts, nextCursor: 'cursor' };
    expect(new TextEncoder().encode(JSON.stringify(result)).length).toBeLessThan(262_144);
    expect(() => validateSessionDataResult(result, largeRequest)).not.toThrow();
  });
  it('mock captures initial and unchanged reconnect but excludes guests', () => {
    for (const trackingMode of ['saved', 'guest'] as const) {
      const backend = new MockBackend();
      backend.handle({ id: 1, method: 'bluetooth.scan', params: {} });
      backend.handle({ id: 2, method: 'device.connect', params: { deviceId: 'mock-board', trackingMode } });
      const active = (backend.handle({ id: 3, method: 'state.get', params: {} }) as { result: AppState }).result
        .activeConnection!;
      const id = active.activeSession?.sessionId;
      if (id) {
        expect(backend.contextRows(id)).toHaveLength(1);
        backend.simulateConnectionState('reconnecting');
        backend.simulateConnectionState('ready');
        expect(backend.contextRows(id)).toHaveLength(2);
        expect(backend.contextRows(id)[0].context).toEqual(backend.contextRows(id)[1].context);
      } else expect(trackingMode).toBe('guest');
    }
  });
});
