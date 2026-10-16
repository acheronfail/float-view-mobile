import { render } from 'svelte/server';
import { describe, expect, it, vi } from 'vitest';
import type { PermissionRow } from '$lib/backend/contract';
import { APP_HEADER_CONTEXT } from '$lib/app/header';
import HomeView from './HomeView.svelte';

const { state } = vi.hoisted(() => ({
  state: {
    status: 'connected',
    busy: false,
    error: null,
    scanError: null as string | null,
    permissionBusy: false,
    scanPermissionPending: false,
    snapshot: {
      permissions: [] as PermissionRow[],
      bluetooth: { radio: 'powered_on', scanPermission: 'granted', connectPermission: 'granted', scanning: false },
    },
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

it('explains disabled Bluetooth without showing stale boards', () => {
  state.snapshot.bluetooth.radio = 'powered_off';
  const body = renderDiscovery();
  expect(body).toContain('Bluetooth is off');
  expect(body).toContain('Turn on Bluetooth');
  expect(body).toContain('Scanning will resume automatically');
  expect(body).toContain('saved boards and sessions');
  expect(body).not.toContain('Another board');
  expect(body).not.toContain('Looking for VESC');
  state.snapshot.bluetooth.radio = 'powered_on';
  expect(renderDiscovery()).toContain('Another board');
});

it('explains first-use access and pending permission rather than claiming scan readiness', () => {
  state.snapshot.bluetooth.radio = 'unknown';
  state.snapshot.bluetooth.scanPermission = 'not_determined';
  expect(renderDiscovery()).toContain('Allow Float View to use Bluetooth when prompted');
  expect(renderDiscovery()).not.toContain('Another board');
  state.scanPermissionPending = true;
  expect(renderDiscovery()).toContain('Waiting for Bluetooth access');
  state.scanPermissionPending = false;
  state.snapshot.bluetooth.scanPermission = 'granted';
  expect(renderDiscovery()).toContain('Checking Bluetooth');
  state.snapshot.bluetooth.radio = 'powered_on';
});

it('offers a scan retry with actionable failure text and hides stale discoveries', () => {
  state.scanError = 'Scanning is temporarily unavailable.';
  const body = renderDiscovery();
  expect(body).toContain('Could not scan for boards');
  expect(body).toContain('Try scanning again');
  expect(body).not.toContain('Another board');
  expect(body).not.toContain('Keep your board nearby');
  state.scanError = null;
});

it('puts saved boards first while preserving discovery order within each group', () => {
  const original = state.discoveredDevices;
  state.discoveredDevices = [
    original[1],
    { deviceId: 'board-4', name: 'Second saved', savedDeviceId: 'saved-4', rssi: -80 },
    original[2],
    original[0],
  ];
  try {
    const body = renderDiscovery();
    const titles = [...body.matchAll(/<span class="block font-medium">(.*?)<\/span>/g)].map((match) => match[1]);
    expect(titles).toEqual(['Second saved', 'XRV', 'Another board', 'board-3']);
    expect(state.discoveredDevices[0].deviceId).toBe('board-2');
    const cards = body.match(/<button\b[\s\S]*?<\/button>/g)!;
    for (const card of cards.slice(0, 2)) {
      expect(card).toContain('Saved board');
      expect(card).toContain('border-accent bg-accent-soft');
    }
    for (const card of cards.slice(2)) {
      expect(card).not.toContain('Saved board');
      expect(card).toContain('border-transparent bg-elevated');
    }
  } finally {
    state.discoveredDevices = original;
  }
});
