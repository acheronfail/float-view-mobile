import { describe, expect, it } from 'vitest';
import type {
	BluetoothState,
	HelloResult,
	Snapshot
} from '../../../../api/generated/typescript/models.js';
import type { ApiClient } from './client';
import { ApiStore, permissionOutcome } from './store.svelte';

function state(scanPermission: BluetoothState['scanPermission']): BluetoothState {
	return {
		radio: 'powered_on',
		scanPermission,
		connectPermission: 'granted'
	};
}

describe('permissionOutcome', () => {
	it.each([
		['granted', 'ready'],
		['not_required', 'ready'],
		['not_determined', 'pending'],
		['unknown', 'pending'],
		['denied', 'denied'],
		['restricted', 'restricted']
	] as const)('maps %s to %s', (permission, outcome) => {
		expect(permissionOutcome(state(permission))).toBe(outcome);
	});
});

function snapshot(active: boolean): Snapshot {
	return {
		revision: 1,
		bluetooth: state('granted'),
		knownDevices: [],
		activeConnection: active ? { deviceId: 'opaque', name: 'Board', state: 'ready' } : null,
		telemetrySubscriptions: [],
		loggingJobs: []
	};
}

class FakeClient {
	isOpen = true;
	active = true;
	readonly methods: string[] = [];

	async hello(): Promise<HelloResult> {
		return {
			api: { major: 1, minor: 0 },
			backend: { name: 'test', version: '1', platform: 'ios' },
			capabilities: [],
			limits: {
				minTelemetryIntervalMs: 50,
				maxMessageBytes: 65_536,
				maxReconnectDurationMs: 3_600_000
			},
			snapshot: snapshot(this.active)
		};
	}

	async request(method: string): Promise<unknown> {
		this.methods.push(method);
		switch (method) {
			case 'savedDevices.list':
				return [];
			case 'sessions.list':
				return { items: [], nextCursor: null };
			case 'system.getSnapshot':
				return snapshot(this.active);
			case 'device.disconnect':
				this.active = false;
				return { accepted: true };
			default:
				throw new Error(`Unexpected method: ${method}`);
		}
	}

	close(): void {
		this.isOpen = false;
	}
}

describe('ApiStore transport recovery', () => {
	it('reconnects a closed foreground socket before sending an explicit disconnect', async () => {
		const first = new FakeClient();
		const replacement = new FakeClient();
		const clients = [first, replacement];
		const store = new ApiStore(
			async () => ({ apiVersion: { major: 1, minor: 0 }, webSocketUrl: 'ws://test' }),
			async () => clients.shift() as unknown as ApiClient
		);

		await store.connect();
		first.isOpen = false;
		await store.disconnectDevice();

		expect(replacement.methods).toContain('device.disconnect');
		expect(store.snapshot?.activeConnection).toBeNull();
		expect(store.error).toBeNull();
		store.close();
	});
});
