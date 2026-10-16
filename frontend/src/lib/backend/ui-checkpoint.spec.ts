import { expect, it } from 'vitest';
import { MockBackend } from '../../../backend-mock/api-v1';
import { validUiCheckpoint } from './ui-checkpoint';

it('validates an explicit, closed session return destination', () => {
  for (const returnTo of ['sessions', 'live', 'boardHistory']) {
    expect(
      validUiCheckpoint({
        version: 1,
        route: { kind: 'session', sessionId: 'ride', savedDeviceId: 'board', returnTo },
        live: null,
        surface: null,
      })
    ).toBe(true);
  }
  for (const route of [
    { kind: 'session', sessionId: 'ride', savedDeviceId: null },
    { kind: 'session', sessionId: 'ride', savedDeviceId: null, returnTo: 'boardHistory' },
    { kind: 'session', sessionId: 'ride', savedDeviceId: null, returnTo: 'https://example.test' },
  ])
    expect(validUiCheckpoint({ version: 1, route, live: null, surface: null })).toBe(false);
});

it('retains only validated presentation without publishing state', () => {
  const events: unknown[] = [];
  const backend = new MockBackend((event) => events.push(event));
  const good = {
    version: 1,
    route: { kind: 'live' },
    live: { deviceId: 'board', mode: 'detailed' },
    surface: { kind: 'boardConfiguration', savedDeviceId: 'saved', batteryOnly: true },
  };
  expect(validUiCheckpoint(good)).toBe(true);
  expect(backend.handle({ id: 1, method: 'uiState.set', params: { checkpoint: good } })).toMatchObject({
    result: { accepted: true },
  });
  for (const bad of [
    { ...good, version: 2 },
    { ...good, draft: {} },
    { ...good, route: { kind: 'settings' } },
    { ...good, live: { deviceId: 'é'.repeat(257), mode: 'normal' } },
    { ...good, surface: { kind: 'shutdown' } },
  ]) {
    expect(validUiCheckpoint(bad)).toBe(false);
    expect(backend.handle({ id: 2, method: 'uiState.set', params: { checkpoint: bad } })).toMatchObject({
      error: { code: 'invalid_request' },
    });
  }
  expect(backend.handle({ id: 3, method: 'uiState.get', params: {} })).toMatchObject({ result: good });
  expect(events).toEqual([]);
  backend.handle({ id: 4, method: 'uiState.set', params: { checkpoint: null } });
  expect(backend.handle({ id: 5, method: 'uiState.get', params: {} })).toMatchObject({ result: null });
  expect(new MockBackend().handle({ id: 6, method: 'uiState.get', params: {} })).toMatchObject({ result: null });
});
