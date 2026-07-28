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
	profile = {
		savedDeviceId: 'saved-board',
		revision: 0,
		values: {} as Record<string, unknown>,
		missingFields: ['seriesCount', 'parallelCount', 'motor']
	};
	settings = {
		revision: 0,
		idleProbeIntervalMs: 500,
		idlePersistIntervalMs: 1000,
		ridingPersistIntervalMs: 100,
		postRideTailMs: 15_000,
		reconnectDurationMs: 300_000,
		automaticSessionLinkingEnabled: false,
		sessionLinkWindowMs: 7_200_000,
		rawPacketCaptureEnabled: false
	};
	deleted = false;

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

	async request(method: string, params?: unknown): Promise<unknown> {
		this.methods.push(method);
		switch (method) {
			case 'savedDevices.list':
				return [
					{
						savedDeviceId: 'saved-board',
						deviceHistoryId: 'history-board',
						displayName: 'Board',
						profileComplete: this.profile.missingFields.length === 0
					}
				];
			case 'boardProfiles.get':
				return this.profile;
			case 'boardProfiles.update': {
				const request = params as {
					expectedRevision: number;
					patch: Record<string, unknown>;
				};
				if (request.expectedRevision !== this.profile.revision) {
					throw new Error('Profile revision conflict');
				}
				this.profile = {
					...this.profile,
					revision: this.profile.revision + 1,
					values: { ...this.profile.values, ...request.patch },
					missingFields: this.profile.missingFields.filter((field) => !(field in request.patch))
				};
				return this.profile;
			}
			case 'sessions.list':
				return {
					items: this.deleted
						? []
						: [
								{
									sessionId: 'session-1',
									deviceHistoryId: 'history-board',
									displayName: 'Board',
									startedAt: '2026-01-01T00:00:00Z',
									endedAt: '2026-01-01T00:01:00Z',
									state: 'completed',
									integrity: 'complete',
									sampleCount: 600,
									eventCount: 0,
									ridingElapsedMs: 50_000,
									totalElapsedMs: 60_000,
									distanceM: 300
								}
							],
					nextCursor: null
				};
			case 'sessionGroups.list':
				return [];
			case 'settings.get':
				return this.settings;
			case 'settings.update': {
				const request = params as {
					expectedRevision: number;
					patch: Record<string, unknown>;
				};
				if (request.expectedRevision !== this.settings.revision) {
					throw new Error('Settings revision conflict');
				}
				this.settings = {
					...this.settings,
					...request.patch,
					revision: this.settings.revision + 1
				};
				return this.settings;
			}
			case 'sessions.getSummary':
				return {
					sessionId: 'session-1',
					deviceHistoryId: 'history-board',
					displayName: 'Board',
					startedAt: '2026-01-01T00:00:00Z',
					endedAt: '2026-01-01T00:01:00Z',
					state: 'completed',
					integrity: 'complete',
					sampleCount: 600,
					eventCount: 0,
					ridingElapsedMs: 50_000,
					totalElapsedMs: 60_000,
					distanceM: 300,
					maxSpeedMps: 8,
					maxDutyCycle: 0.8
				};
			case 'sessions.querySeries':
				return [
					{ elapsedMs: 0, values: { packVoltageV: 60, dutyCycle: 0.1, speedKph: 0 } },
					{ elapsedMs: 60_000, values: { packVoltageV: 55, dutyCycle: 0.8, speedKph: 20 } }
				];
			case 'sessions.getDiagnosticWindow':
				return [{ elapsedMs: 30_000, values: { packVoltageV: 57 } }];
			case 'sessions.export':
				return { exportId: 'export-1', fileName: 'session-1.csv', delivery: 'presented' };
			case 'sessions.delete':
				this.deleted = true;
				return { accepted: true };
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

	it('loads and revision-updates a local Board Profile', async () => {
		const client = new FakeClient();
		const store = new ApiStore(
			async () => ({ apiVersion: { major: 1, minor: 0 }, webSocketUrl: 'ws://test' }),
			async () => client as unknown as ApiClient
		);

		await store.connect();
		await store.openBoardProfile('saved-board');
		await store.updateBoardProfile({
			seriesCount: 18,
			parallelCount: 2,
			motor: 'hypercore_xr'
		});

		expect(store.selectedBoardProfile?.revision).toBe(1);
		expect(store.selectedBoardProfile?.values).toMatchObject({
			seriesCount: 18,
			parallelCount: 2,
			motor: 'hypercore_xr'
		});
		expect(store.savedDevices[0]?.profileComplete).toBe(true);
		expect(client.methods).toContain('boardProfiles.update');
		store.close();
	});

	it('loads bounded overview and diagnostic data for review', async () => {
		const client = new FakeClient();
		const store = new ApiStore(
			async () => ({ apiVersion: { major: 1, minor: 0 }, webSocketUrl: 'ws://test' }),
			async () => client as unknown as ApiClient
		);
		await store.connect();
		await store.openSession('session-1');
		await store.loadDiagnosticWindow(30_000);

		expect(store.selectedSession?.sessionId).toBe('session-1');
		expect(store.sessionSeries).toHaveLength(2);
		expect(store.diagnosticSeries).toHaveLength(1);
		expect(client.methods).toContain('sessions.querySeries');
		expect(client.methods).toContain('sessions.getDiagnosticWindow');
		store.close();
	});

	it('revision-updates linking settings and manages a selected session', async () => {
		const client = new FakeClient();
		const store = new ApiStore(
			async () => ({ apiVersion: { major: 1, minor: 0 }, webSocketUrl: 'ws://test' }),
			async () => client as unknown as ApiClient
		);
		await store.connect();
		await store.updateSessionSettings({ automaticSessionLinkingEnabled: true });
		await store.openSession('session-1');
		await store.exportSession('csv');

		expect(store.settings?.revision).toBe(1);
		expect(store.settings?.automaticSessionLinkingEnabled).toBe(true);
		expect(store.exportStatus).toContain('session-1.csv');

		await store.deleteSelectedSession();
		expect(store.selectedSession).toBeNull();
		expect(store.sessions).toHaveLength(0);
		expect(client.methods).toContain('sessions.export');
		expect(client.methods).toContain('sessions.delete');
		store.close();
	});
});
