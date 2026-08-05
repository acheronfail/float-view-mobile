import { describe, expect, it } from 'vitest';
import type {
  AppState,
  BackendMethod,
  BackendParams,
  BackendResult,
  BluetoothPermissionState,
  BluetoothState,
} from '$lib/backend/contract';
import type { ApiClient } from './client';
import { BackendState, permissionOutcome } from './state.svelte';

const bluetooth = (scanPermission: BluetoothPermissionState): BluetoothState => ({
  radio: 'powered_on',
  scanPermission,
  connectPermission: 'granted',
  scanning: false,
});

const initial: AppState = {
  revision: 1,
  bluetooth: bluetooth('granted'),
  discoveredDevices: [],
  savedDevices: [
    {
      savedDeviceId: 'saved-board',
      deviceHistoryId: 'history-board',
      revision: 0,
      displayName: 'Advertised',
      customDisplayName: null,
      advertisedName: 'Advertised',
      bluetoothIdentifier: 'AA:BB:CC:DD:EE:FF',
      profileComplete: false,
      summary: {
        sessionCount: 0,
        totalRidingElapsedMs: 0,
        totalDistanceM: null,
        mostRecentSessionAt: null,
      },
    },
  ],
  activeConnection: null,
  sessionCatalogRevision: 0,
  location: {
    status: 'disabled',
    permissionAction: 'requestPermission',
    backgroundCapture: 'unavailable',
    reason: null,
  },
  settings: {
    revision: 0,
    idleProbeIntervalMs: 500,
    idlePersistIntervalMs: 1_000,
    ridingPersistIntervalMs: 100,
    postRideTailMs: 5_000,
    reconnectDurationMs: 300_000,
    minimumSessionDistance: 100,
    themePreference: 'system',
    unitPreference: 'metric',
    historyGapThresholdSeconds: null,
    locationRecordingEnabled: false,
    includeGpsLocationWhenExporting: false,
  },
};

class FakeClient {
  isOpen = true;
  state = structuredClone(initial);
  locationRequests: string[] = [];
  private handler?: (state: AppState) => void;

  async request<M extends BackendMethod>(method: M, params: BackendParams<M>): Promise<BackendResult<M>> {
    let result: unknown;
    switch (method) {
      case 'state.get':
        result = this.state;
        break;
      case 'settings.update':
        this.state = {
          ...this.state,
          revision: this.state.revision + 1,
          settings: {
            ...this.state.settings,
            ...(params as BackendParams<'settings.update'>).patch,
            revision: this.state.settings.revision + 1,
          },
        };
        this.handler?.(this.state);
        result = this.state.settings;
        break;
      case 'device.disconnect':
        result = { accepted: true };
        break;
      case 'location.requestPermission':
        this.locationRequests.push(method);
        this.state = {
          ...this.state,
          location: {
            status: 'authorized',
            permissionAction: null,
            backgroundCapture: 'available',
            reason: null,
          },
        };
        result = this.state.location;
        break;
      case 'location.openSettings':
        this.locationRequests.push(method);
        this.state = {
          ...this.state,
          location: {
            status: 'authorized',
            permissionAction: null,
            backgroundCapture: 'available',
            reason: null,
          },
        };
        result = this.state.location;
        break;
      case 'savedDevice.update': {
        const update = params as BackendParams<'savedDevice.update'>;
        const current = this.state.savedDevices[0];
        const customDisplayName = update.customDisplayName?.trim() || null;
        const saved = {
          ...current,
          revision: current.revision + 1,
          customDisplayName,
          displayName: customDisplayName ?? current.advertisedName ?? current.bluetoothIdentifier ?? 'Board',
        };
        this.state = { ...this.state, savedDevices: [saved] };
        result = saved;
        break;
      }
      case 'session.get':
        result = {
          summary: {
            sessionId: 'session-1',
            deviceHistoryId: 'history-board',
            displayName: 'Board',
            startedAt: '2026-07-30T00:00:00Z',
            endedAt: '2026-07-30T00:01:00Z',
            state: 'completed',
            integrity: 'complete',
            sampleCount: 2,
            eventCount: 1,
            ridingElapsedMs: 1_000,
            totalElapsedMs: 1_000,
          },
          fields: [{ name: 'fieldWeakeningCurrentA', source: 'stable' }],
          eventCount: 1,
          recordedSampleCount: 2,
          storedSampleCount: 2,
          prune: {
            targetResolutionSeconds: null,
            recordedSampleCount: 2,
            storedSampleCount: 2,
            policy: null,
          },
          snapshot: {
            historyRevision: 0,
            committedSampleSequence: 1,
            committedEventSequence: 0,
            committedElapsedMs: 1_000,
            committedLocationSequence: 0,
            committedLocationElapsedMs: 0,
          },
          location: { available: false, count: 0, lastSequence: 0, lastElapsedMs: 0, status: 'disabled' },
        };
        break;
      default:
        throw new Error(`Unexpected method ${method}`);
    }
    return result as BackendResult<M>;
  }
  onStateChanged(handler: (state: AppState) => void) {
    this.handler = handler;
    return () => {
      this.handler = undefined;
    };
  }
  emitState() {
    this.handler?.(this.state);
  }
  close() {
    this.isOpen = false;
  }
}

describe('BackendState', () => {
  it.each([
    ['granted', 'ready'],
    ['not_required', 'ready'],
    ['not_determined', 'pending'],
    ['denied', 'denied'],
    ['restricted', 'restricted'],
  ] as const)('maps %s permission to %s', (permission, outcome) => {
    expect(permissionOutcome(bluetooth(permission))).toBe(outcome);
  });

  it('loads one authoritative state and accepts only newer events', async () => {
    const client = new FakeClient();
    const store = new BackendState(
      async () => ({ webSocketUrl: 'ws://test' }),
      async () => client as unknown as ApiClient
    );
    await store.connect();
    expect(store.snapshot?.revision).toBe(1);
    await store.updateSessionSettings({
      minimumSessionDistance: 250,
    });
    expect(store.settings?.minimumSessionDistance).toBe(250);
    expect(store.snapshot?.revision).toBe(2);
    store.close();
  });

  it('enables location with an immediate permission request and routes denied state to settings', async () => {
    const client = new FakeClient();
    const store = new BackendState(
      async () => ({ webSocketUrl: 'ws://test' }),
      async () => client as unknown as ApiClient
    );
    await store.connect();
    await store.updateSessionSettings({ locationRecordingEnabled: true });
    expect(client.locationRequests).toEqual(['location.requestPermission']);
    expect(store.snapshot?.location.status).toBe('authorized');

    client.state = {
      ...client.state,
      location: {
        status: 'denied',
        permissionAction: 'openSettings',
        backgroundCapture: 'unavailable',
        reason: 'Enable location in Settings.',
      },
    };
    client.emitState();
    await store.requestLocationPermission();
    expect(client.locationRequests.at(-1)).toBe('location.openSettings');
    store.close();
  });

  it('reconciles custom board names and clears back to advertised identity', async () => {
    const client = new FakeClient();
    const store = new BackendState(
      async () => ({ webSocketUrl: 'ws://test' }),
      async () => client as unknown as ApiClient
    );
    await store.connect();
    await store.updateSavedDeviceName('saved-board', 'Trail board');
    expect(store.savedDevices[0]).toMatchObject({ displayName: 'Trail board', revision: 1 });
    await store.updateSavedDeviceName('saved-board', null);
    expect(store.savedDevices[0]).toMatchObject({ displayName: 'Advertised', revision: 2 });
    store.close();
  });

  it('loads bounded session metadata without embedding history', async () => {
    const client = new FakeClient();
    const store = new BackendState(
      async () => ({ webSocketUrl: 'ws://test' }),
      async () => client as unknown as ApiClient
    );
    await store.connect();
    expect(await store.openSession('session-1')).toBe(true);
    expect(store.sessionMetadata?.fields).toEqual([{ name: 'fieldWeakeningCurrentA', source: 'stable' }]);
    expect(store.sessionMetadata?.recordedSampleCount).toBe(2);
    store.closeSession();
    expect(store.sessionMetadata).toBeNull();
    store.close();
  });
});
