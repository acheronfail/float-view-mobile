import { describe, expect, it } from 'vitest';
import type {
	AppState,
	BackendMethod,
	BackendParams,
	BackendResult,
	BluetoothPermissionState,
	BluetoothState
} from '$lib/backend/contract';
import type { ApiClient } from './client';
import { BackendState, permissionOutcome } from './state.svelte';

const bluetooth = (scanPermission: BluetoothPermissionState): BluetoothState => ({
	radio: 'powered_on',
	scanPermission,
	connectPermission: 'granted',
	scanning: false
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
			profileComplete: false
		}
	],
	activeConnection: null,
	recentSessions: [],
	sessionGroups: [],
	settings: {
		revision: 0,
		idleProbeIntervalMs: 500,
		idlePersistIntervalMs: 1_000,
		ridingPersistIntervalMs: 100,
		postRideTailMs: 5_000,
		reconnectDurationMs: 300_000,
		automaticSessionLinkingEnabled: false,
		sessionLinkWindowMs: 7_200_000,
		themePreference: 'system',
		unitPreference: 'metric'
	}
};

class FakeClient {
	isOpen = true;
	state = structuredClone(initial);
	private handler?: (state: AppState) => void;

	async request<M extends BackendMethod>(
		method: M,
		params: BackendParams<M>
	): Promise<BackendResult<M>> {
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
						revision: this.state.settings.revision + 1
					}
				};
				this.handler?.(this.state);
				result = this.state.settings;
				break;
			case 'device.disconnect':
				result = { accepted: true };
				break;
			case 'savedDevice.update': {
				const update = params as BackendParams<'savedDevice.update'>;
				const current = this.state.savedDevices[0];
				const customDisplayName = update.customDisplayName?.trim() || null;
				const saved = {
					...current,
					revision: current.revision + 1,
					customDisplayName,
					displayName: customDisplayName ?? current.advertisedName ?? current.bluetoothIdentifier
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
						totalElapsedMs: 1_000
					},
					series: [{ elapsedMs: 0, values: { fieldWeakeningCurrentA: 4 } }],
					events: [{ elapsedMs: 500, kind: 'stopCondition', value: 'quickStop' }],
					eventsTruncated: true
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
		['restricted', 'restricted']
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
		await store.updateSessionSettings({ automaticSessionLinkingEnabled: true });
		expect(store.settings?.automaticSessionLinkingEnabled).toBe(true);
		expect(store.snapshot?.revision).toBe(2);
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

	it('recovers bounded historical series and typed transitions together', async () => {
		const client = new FakeClient();
		const store = new BackendState(
			async () => ({ webSocketUrl: 'ws://test' }),
			async () => client as unknown as ApiClient
		);
		await store.connect();
		expect(await store.openSession('session-1')).toBe(true);
		expect(store.sessionSeries).toEqual([
			{ elapsedMs: 0, values: { fieldWeakeningCurrentA: 4 } }
		]);
		expect(store.sessionEvents).toEqual([
			{ elapsedMs: 500, kind: 'stopCondition', value: 'quickStop' }
		]);
		expect(store.sessionEventsTruncated).toBe(true);
		store.closeSession();
		expect(store.sessionEvents).toEqual([]);
		expect(store.sessionEventsTruncated).toBe(false);
		store.close();
	});
});
