import { describe, expect, it, vi } from 'vitest';
import { UiRestoration, resolveUiCheckpoint, uiHref, uiRoute } from './ui-restoration.svelte';
import type { AppState, UiCheckpoint } from '$lib/backend/contract';
import { MockBackend } from '../../../backend-mock/api-v1';

const checkpoint = (kind: 'settings' | 'sessions'): UiCheckpoint => ({
  version: 1,
  route: { kind },
  live: null,
  surface: null,
});
describe('UI restoration', () => {
  it('restores Custom only for the same saved identity, while retaining existing normal checkpoints', () => {
    const backend = new MockBackend(() => {}, { safetyDisclaimerAccepted: true });
    backend.handle({ id: 1, method: 'bluetooth.requestPermission', params: {} });
    backend.handle({ id: 2, method: 'bluetooth.scan', params: { timeoutMs: 100 } });
    backend.handle({ id: 3, method: 'device.connect', params: { deviceId: 'mock-board', trackingMode: 'saved' } });
    const state = (backend.handle({ id: 4, method: 'state.get', params: {} }) as { result: AppState }).result;
    state.activeConnection!.deviceId = 'board';
    state.activeConnection!.savedDeviceId = 'first';
    const custom: UiCheckpoint = {
      version: 1,
      route: { kind: 'live' },
      live: { deviceId: 'board', savedDeviceId: 'first', mode: 'custom' },
      surface: null,
    };
    expect(resolveUiCheckpoint(custom, state).live).toEqual(custom.live);
    state.activeConnection!.savedDeviceId = 'second';
    expect(resolveUiCheckpoint(custom, state).live).toBeNull();
    expect(resolveUiCheckpoint({ ...custom, live: { deviceId: 'board', mode: 'normal' } }, state).live?.mode).toBe(
      'normal'
    );
    state.activeConnection!.trackingMode = 'guest';
    state.activeConnection!.savedDeviceId = null;
    expect(resolveUiCheckpoint(custom, state).live).toBeNull();
  });
  it('restores read-only licences to Settings without extending the wire route', () => {
    expect(uiRoute('/settings/licences', {}, new URL('https://example.test/#/settings/licences'))).toEqual({
      kind: 'settings',
    });
  });
  it('round trips the session entry route independently of recording state', () => {
    for (const [query, returnTo, savedDeviceId] of [
      ['', 'sessions', null],
      ['?from=live', 'live', null],
      ['?board=a%2Fb', 'boardHistory', 'a/b'],
    ] as const) {
      const route = uiRoute(
        '/sessions/[sessionId]',
        { sessionId: 'ride' },
        new URL(`https://example.test/#/sessions/ride${query}`)
      );
      expect(route).toEqual({ kind: 'session', sessionId: 'ride', savedDeviceId, returnTo });
      expect(uiHref(route)).toBe(`#/sessions/ride${query}`);
    }
  });
  it('deduplicates and retains only the latest queued choice', async () => {
    let finish!: () => void;
    const save = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            finish = resolve;
          })
      )
      .mockResolvedValue(undefined);
    const owner = new UiRestoration(save);
    owner.capture(checkpoint('settings'));
    owner.capture(checkpoint('settings'));
    owner.capture(checkpoint('sessions'));
    owner.capture(checkpoint('settings'));
    owner.capture(checkpoint('sessions'));
    expect(save).toHaveBeenCalledTimes(1);
    finish();
    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(2));
    expect(save.mock.calls[1][0]).toEqual(checkpoint('sessions'));
    owner.dispose();
    owner.capture(checkpoint('settings'));
    expect(save).toHaveBeenCalledTimes(2);
  });
  it('does not retry failures periodically and can save the next meaningful choice', async () => {
    const save = vi.fn().mockRejectedValueOnce(new Error('closed')).mockResolvedValue(undefined);
    const owner = new UiRestoration(save);
    owner.capture(checkpoint('settings'));
    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(1));
    await Promise.resolve();
    owner.capture(checkpoint('settings'));
    expect(save).toHaveBeenCalledTimes(1);
    owner.capture(checkpoint('sessions'));
    await vi.waitFor(() => expect(save).toHaveBeenCalledTimes(2));
    owner.dispose();
  });
  it('serializes reactive presentation as an ordinary wire record', async () => {
    const save = vi.fn().mockResolvedValue(undefined);
    const owner = new UiRestoration(save);
    owner.pending = checkpoint('settings');
    owner.capture(owner.pending);
    expect(save.mock.calls[0][0]).toEqual(checkpoint('settings'));
    owner.dispose();
  });
  it('resolves stale board/device targets without changing unrelated destinations', () => {
    const backend = new MockBackend();
    const state = (backend.handle({ id: 1, method: 'state.get', params: {} }) as { result: AppState }).result;
    expect(resolveUiCheckpoint(checkpoint('settings'), state)).toEqual(checkpoint('settings'));
    const missing: UiCheckpoint = {
      version: 1,
      route: { kind: 'boardHistory', savedDeviceId: 'removed' },
      live: null,
      surface: { kind: 'boardActions' },
    };
    expect(resolveUiCheckpoint(missing, state).route).toEqual({ kind: 'sessions' });
    const live: UiCheckpoint = {
      version: 1,
      route: { kind: 'live' },
      live: { deviceId: 'old', mode: 'detailed' },
      surface: { kind: 'liveCharts' },
    };
    expect(resolveUiCheckpoint(live, state)).toEqual({
      version: 1,
      route: { kind: 'home' },
      live: null,
      surface: null,
    });
    expect(uiHref({ kind: 'session', sessionId: 'a/b', savedDeviceId: 'board', returnTo: 'boardHistory' })).toBe(
      '#/sessions/a%2Fb?board=board'
    );
  });
});
