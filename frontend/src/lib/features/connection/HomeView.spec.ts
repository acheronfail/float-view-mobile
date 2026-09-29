import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import { APP_HEADER_CONTEXT } from '$lib/app/header';
import HomeView from './HomeView.svelte';

const { state } = vi.hoisted(() => ({
  state: {
    status: 'connected',
    busy: false,
    error: null,
    discoveredDevices: [
      { deviceId: 'board-1', name: 'Bluetooth UART', savedDeviceId: 'saved-1', rssi: -55 },
      { deviceId: 'board-2', name: 'Another board', savedDeviceId: null, rssi: -70 },
      { deviceId: 'board-3', name: '', savedDeviceId: null, rssi: null },
    ],
    savedDevices: [{ savedDeviceId: 'saved-1', displayName: 'XRV' }],
  },
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$lib/backend/state.svelte', () => ({ backendState: state }));

function renderDiscovery() {
  return render(HomeView, {
    context: new Map([[APP_HEADER_CONTEXT, { set: vi.fn(), clear: vi.fn() }]]),
  }).body;
}

describe('discovered board names', () => {
  it('uses the saved display name after disconnecting and keeps unsaved names', () => {
    const body = renderDiscovery();
    expect(body).toContain('XRV');
    expect(body).not.toContain('Bluetooth UART');
    expect(body).toContain('Another board');
    expect(body).toContain('board-3');
  });

  it('uses the current saved name and falls back after forgetting the board', () => {
    state.savedDevices[0].displayName = 'Renamed board';
    expect(renderDiscovery()).toContain('Renamed board');
    state.savedDevices = [];
    expect(renderDiscovery()).toContain('Bluetooth UART');
    state.savedDevices = [{ savedDeviceId: 'saved-1', displayName: 'XRV' }];
  });
});
