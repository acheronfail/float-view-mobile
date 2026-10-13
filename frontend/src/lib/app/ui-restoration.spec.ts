import { describe, expect, it, vi } from 'vitest';
import { UiRestoration, resolveUiCheckpoint, uiHref } from './ui-restoration.svelte';
import type { AppState, UiCheckpoint } from '$lib/backend/contract';
import { MockBackend } from '../../../backend-mock/api-v1';

const checkpoint = (kind: 'settings' | 'sessions'): UiCheckpoint => ({
  version: 1,
  route: { kind },
  live: null,
  surface: null,
});
describe('UI restoration', () => {
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
    expect(uiHref({ kind: 'session', sessionId: 'a/b', savedDeviceId: 'board' })).toBe('#/sessions/a%2Fb?board=board');
  });
});
