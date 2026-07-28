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
import { ApiStore, permissionOutcome } from './store.svelte';

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
	savedDevices: [],
	activeConnection: null,
	recentSessions: [],
	sessionGroups: [],
	settings: {
		revision: 0,
		idleProbeIntervalMs: 500,
		idlePersistIntervalMs: 1_000,
		ridingPersistIntervalMs: 100,
		postRideTailMs: 15_000,
		reconnectDurationMs: 300_000,
		automaticSessionLinkingEnabled: false,
		sessionLinkWindowMs: 7_200_000,
		rawPacketCaptureEnabled: false
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

describe('ApiStore', () => {
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
		const store = new ApiStore(
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
});
