import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import type { AppState } from '$lib/backend/contract';
import { MockBackend } from '../../../../backend-mock/api-v1';
import DetailedView from './DetailedView.svelte';

const { state } = vi.hoisted(() => ({
  state: {
    snapshot: null as AppState | null,
    settings: { unitPreference: 'metric' },
  },
}));
vi.mock('$lib/backend/state.svelte', () => ({ backendState: state }));

function connected(can: boolean) {
  const backend = new MockBackend();
  backend.handle({ id: 1, method: 'bluetooth.scan', params: {} });
  backend.handle({ id: 2, method: 'device.connect', params: { deviceId: 'mock-board', trackingMode: 'guest' } });
  if (can) backend.simulateCanReady(1);
  state.snapshot = (backend.handle({ id: 3, method: 'state.get', params: {} }) as { result: AppState }).result;
}

describe('detailed firmware information', () => {
  it('shows a separate Bluetooth gateway for CAN connections', () => {
    connected(true);
    const body = render(DetailedView).body;
    expect(body).toContain('Motor controller firmware');
    expect(body).toContain('Thor300');
    expect(body).toContain('CAN 1');
    expect(body).toContain('Bluetooth gateway');
    expect(body).toContain('VESC Express T');
    expect(body).toContain('Firmware name:');
    expect(body).toContain('Stable');
  });

  it('shows firmware once for a direct Bluetooth controller', () => {
    connected(false);
    const body = render(DetailedView).body;
    expect(body).toContain('aria-label="Firmware"');
    expect(body).not.toContain('Bluetooth gateway');
    expect(body).not.toContain('Motor controller firmware');
  });

  it('distinguishes test releases and handles unavailable gateway identity', () => {
    connected(true);
    state.snapshot!.activeConnection!.transportFirmware!.testVersion = 7;
    expect(render(DetailedView).body).toContain('Test version 7');
    state.snapshot!.activeConnection!.transportFirmware = null;
    expect(render(DetailedView).body).not.toContain('Bluetooth gateway');
  });
});
