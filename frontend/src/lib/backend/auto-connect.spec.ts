import { afterEach, describe, expect, it, vi } from 'vitest';
import { MockBackend } from '../../../backend-mock/api-v1';
import type { AppState, DiscoveredDevice } from './contract';

const state = (backend: MockBackend) =>
  (backend.handle({ id: 1, method: 'state.get', params: {} }) as { result: AppState }).result;
const settings = (backend: MockBackend, patch: Record<string, unknown>) =>
  backend.handle({
    id: 2,
    method: 'settings.update',
    params: { expectedRevision: state(backend).settings.revision, patch },
  });
const preferred: DiscoveredDevice = {
  deviceId: 'mock-board',
  name: 'Board',
  state: 'discovered',
  rssi: -50,
  savedDeviceId: 'saved-board',
};
const create = () =>
  new MockBackend(() => {}, {
    safetyDisclaimerAccepted: true,
    autoConnectSavedDeviceId: 'saved-board',
    advertising: false,
  });

afterEach(() => vi.useRealTimers());

describe('foreground auto-connect', () => {
  it('accepts only a fresh matching saved discovery and consumes eligibility', () => {
    vi.useFakeTimers();
    const backend = create();
    expect(state(backend).bluetooth.scanning).toBe(true);
    backend.simulateDiscovery({ ...preferred, savedDeviceId: null });
    expect(state(backend).activeConnection).toBeNull();
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection).toMatchObject({ trackingMode: 'saved', savedDeviceId: 'saved-board' });
    backend.handle({ id: 3, method: 'device.disconnect', params: {} });
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection).toBeNull();
  });

  it('expires at 15 seconds even when Home scans later', () => {
    vi.useFakeTimers();
    const backend = create();
    vi.advanceTimersByTime(15_000);
    backend.handle({ id: 3, method: 'bluetooth.scan', params: { timeoutMs: 10_000 } });
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection).toBeNull();
  });

  it('rearms only on a new foreground visit after manual failure', () => {
    vi.useFakeTimers();
    const backend = create();
    backend.handle({ id: 3, method: 'device.connect', params: { deviceId: 'absent', trackingMode: 'saved' } });
    backend.simulateForeground(true);
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection).toBeNull();
    backend.simulateForeground(false);
    backend.simulateForeground(true);
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection?.trackingMode).toBe('saved');
  });

  it('waits for disclaimer acceptance and never rearms on settings reads', () => {
    vi.useFakeTimers();
    const backend = new MockBackend(() => {}, { autoConnectSavedDeviceId: 'saved-board', advertising: false });
    expect(state(backend).bluetooth.scanning).toBe(false);
    settings(backend, { safetyDisclaimerAccepted: true });
    expect(state(backend).bluetooth.scanning).toBe(true);
    backend.simulateForeground(false);
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection).toBeNull();
  });

  it('resumes radio discovery without extending the original window', () => {
    vi.useFakeTimers();
    const backend = create();
    vi.advanceTimersByTime(5000);
    backend.simulateBluetooth({ radio: 'powered_off' });
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection).toBeNull();
    expect(state(backend).bluetooth.scanning).toBe(false);
    vi.advanceTimersByTime(9000);
    backend.simulateBluetooth({ radio: 'powered_on' });
    expect(state(backend).bluetooth.scanning).toBe(true);
    vi.advanceTimersByTime(1000);
    backend.simulateDiscovery(preferred);
    expect(state(backend).activeConnection).toBeNull();
  });

  it('validates preference atomically and clears it when the selected board is removed', () => {
    const backend = new MockBackend();
    const before = state(backend).settings;
    expect(settings(backend, { autoConnectSavedDeviceId: 'missing', unitPreference: 'imperial' })).toMatchObject({
      error: { code: 'not_found' },
    });
    expect(state(backend).settings).toEqual(before);
    expect(settings(backend, { autoConnectSavedDeviceId: 42 })).toMatchObject({ error: { code: 'invalid_request' } });
    settings(backend, { autoConnectSavedDeviceId: 'saved-board' });
    backend.handle({ id: 3, method: 'savedDevice.delete', params: { savedDeviceId: 'saved-board' } });
    expect(state(backend).settings).toMatchObject({ autoConnectSavedDeviceId: null, revision: 2 });
  });
});
