import type {
	BluetoothState,
	BoardProfile,
	Device,
	FirmwareInfo,
	HelloResult,
	SavedDevice,
	Session,
	SessionGroup,
	SeriesPoint,
	Settings,
	Snapshot
} from '../../../../api/generated/typescript/models.js';
import { connectBootstrap, loadBootstrap } from './bootstrap';
import { ApiRequestError, type ApiClient } from './client';

export type ApiConnectionStatus = 'starting' | 'connected' | 'reconnecting' | 'failed';
export type PermissionOutcome = 'ready' | 'pending' | 'denied' | 'restricted';

export function permissionOutcome(state: BluetoothState): PermissionOutcome {
	switch (state.scanPermission) {
		case 'granted':
		case 'not_required':
			return 'ready';
		case 'denied':
			return 'denied';
		case 'restricted':
			return 'restricted';
		default:
			return 'pending';
	}
}

export class ApiStore {
	status = $state<ApiConnectionStatus>('starting');
	snapshot = $state<Snapshot | null>(null);
	hello = $state<HelloResult | null>(null);
	error = $state<string | null>(null);
	savedDevices = $state<SavedDevice[]>([]);
	discoveredDevices = $state<Device[]>([]);
	firmware = $state<FirmwareInfo | null>(null);
	sessions = $state<Session[]>([]);
	sessionGroups = $state<SessionGroup[]>([]);
	settings = $state<Settings | null>(null);
	selectedSession = $state<Session | null>(null);
	sessionSeries = $state<SeriesPoint[]>([]);
	diagnosticSeries = $state<SeriesPoint[]>([]);
	selectedBoardProfile = $state<BoardProfile | null>(null);
	exportStatus = $state<string | null>(null);
	busy = $state(false);
	private client: ApiClient | null = null;

	constructor(
		private readonly bootstrapLoader = loadBootstrap,
		private readonly bootstrapConnector = connectBootstrap
	) {}

	async connect(): Promise<void> {
		this.status = this.client ? 'reconnecting' : 'starting';
		this.client?.close();
		this.client = null;
		try {
			const bootstrap = await this.bootstrapLoader();
			this.client = await this.bootstrapConnector(bootstrap);
			this.hello = await this.client.hello({
				client: { name: 'float-view-web', version: '0.1.0' },
				api: { major: 1, minMinor: 0, maxMinor: 0 }
			});
			this.snapshot = this.hello.snapshot;
			await this.refreshCollections();
			this.status = 'connected';
			this.error = null;
		} catch (error) {
			this.status = 'failed';
			this.error = error instanceof Error ? error.message : String(error);
		}
	}

	async refreshCollections(): Promise<void> {
		if (!this.client) return;
		const [savedDevices, sessions, sessionGroups, settings, snapshot] = await Promise.all([
			this.client.request('savedDevices.list', {}),
			this.client.request('sessions.list', { limit: 50 }),
			this.client.request('sessionGroups.list', {}),
			this.client.request('settings.get', {}),
			this.client.request('system.getSnapshot', {})
		]);
		this.savedDevices = savedDevices;
		this.sessions = sessions.items;
		this.sessionGroups = sessionGroups;
		this.settings = settings;
		this.snapshot = snapshot;
		this.discoveredDevices = snapshot.knownDevices;
	}

	async scan(): Promise<void> {
		await this.run(async (client) => {
			let scan;
			try {
				scan = await client.request('discovery.start', { timeoutMs: 5_000 });
			} catch (error) {
				if (error instanceof ApiRequestError && error.data.kind === 'permission_required') {
					await client.request('bluetooth.requestPermission', {});
					const state = await this.waitForPermissionResolution(client);
					this.updateBluetoothState(state);
					switch (permissionOutcome(state)) {
						case 'ready':
							scan = await client.request('discovery.start', { timeoutMs: 5_000 });
							break;
						case 'denied':
							throw new Error(
								'Bluetooth permission denied. Enable Bluetooth access for Float View in system settings, then scan again.'
							);
						case 'restricted':
							throw new Error('Bluetooth access is restricted by system policy.');
						case 'pending':
							throw new Error('Bluetooth permission requested. Allow it, then scan again.');
					}
				} else {
					throw error;
				}
			}
			await new Promise((resolve) => setTimeout(resolve, 5_000));
			await client.request('discovery.stop', { scanId: scan.scanId });
			await this.refreshCollections();
		});
	}

	private async waitForPermissionResolution(client: ApiClient): Promise<BluetoothState> {
		const deadline = Date.now() + 30_000;
		let state = await client.request('bluetooth.getState', {});
		while (permissionOutcome(state) === 'pending' && Date.now() < deadline) {
			await new Promise((resolve) => setTimeout(resolve, 250));
			state = await client.request('bluetooth.getState', {});
		}
		return state;
	}

	private updateBluetoothState(bluetooth: BluetoothState): void {
		if (this.snapshot) this.snapshot = { ...this.snapshot, bluetooth };
	}

	async connectDevice(deviceId: string): Promise<void> {
		await this.run(async (client) => {
			this.firmware = null;
			await client.request('device.connect', { deviceId, timeoutMs: 10_000 });
			const deadline = Date.now() + 10_000;
			do {
				await this.refreshCollections();
				const state = this.snapshot?.activeConnection?.state;
				if (state === 'ready') {
					await this.loadFirmwareInfo(client);
					return;
				}
				if (state === 'failed' || state === 'disconnected') return;
				if (!this.snapshot?.activeConnection) {
					throw new Error('The board disconnected before its VESC service became ready.');
				}
				await new Promise((resolve) => setTimeout(resolve, 250));
			} while (Date.now() < deadline);
			throw new Error('The board did not finish connecting within 10 seconds.');
		});
	}

	async disconnectDevice(): Promise<void> {
		await this.run(async (client) => {
			await client.request('device.disconnect', {});
			const deadline = Date.now() + 10_000;
			do {
				await this.refreshCollections();
				if (!this.snapshot?.activeConnection) {
					this.firmware = null;
					return;
				}
				if (this.snapshot.activeConnection.state === 'failed') {
					throw new Error('The board did not disconnect cleanly.');
				}
				await new Promise((resolve) => setTimeout(resolve, 250));
			} while (Date.now() < deadline);
			throw new Error('The board did not finish disconnecting within 10 seconds.');
		});
	}

	private async loadFirmwareInfo(client: ApiClient): Promise<void> {
		const deadline = Date.now() + 5_000;
		do {
			try {
				this.firmware = await client.request('vesc.getFirmwareInfo', {});
				return;
			} catch (error) {
				if (!(error instanceof ApiRequestError) || error.data.kind !== 'device_not_ready') {
					throw error;
				}
			}
			await new Promise((resolve) => setTimeout(resolve, 100));
		} while (Date.now() < deadline);
		throw new Error('The VESC did not return firmware information within 5 seconds.');
	}

	async openSession(sessionId: string): Promise<void> {
		await this.run(async (client) => {
			this.exportStatus = null;
			this.selectedSession = await client.request('sessions.getSummary', { sessionId });
			this.diagnosticSeries = [];
			this.sessionSeries = await client.request('sessions.querySeries', {
				sessionId,
				startElapsedMs: 0,
				endElapsedMs: Math.max(0, this.selectedSession.totalElapsedMs),
				fields: ['packVoltageV', 'dutyCycle', 'speedKph'],
				maxPoints: 400
			});
		});
	}

	async updateSessionSettings(patch: Omit<Partial<Settings>, 'revision'>): Promise<void> {
		const current = this.settings;
		if (!current) return;
		await this.run(async (client) => {
			this.settings = await client.request('settings.update', {
				expectedRevision: current.revision,
				patch
			});
		});
	}

	async unlinkSessionGroup(sessionGroupId: string): Promise<void> {
		await this.run(async (client) => {
			await client.request('sessionGroups.unlink', { sessionGroupId });
			await this.refreshCollections();
		});
	}

	async exportSession(format: 'float-view-archive' | 'csv'): Promise<void> {
		const selected = this.selectedSession;
		if (!selected) return;
		await this.run(async (client) => {
			const value = await client.request('sessions.export', {
				sessionId: selected.sessionId,
				format
			});
			this.exportStatus =
				value.delivery === 'presented'
					? `${value.fileName} is ready to save or share.`
					: `${value.fileName} is staged by the test backend.`;
		});
	}

	async deleteSelectedSession(): Promise<void> {
		const selected = this.selectedSession;
		if (!selected) return;
		await this.run(async (client) => {
			await client.request('sessions.delete', { sessionId: selected.sessionId });
			this.closeSession();
			await this.refreshCollections();
		});
	}

	async loadDiagnosticWindow(centerElapsedMs: number): Promise<void> {
		const selected = this.selectedSession;
		if (!selected) return;
		await this.run(async (client) => {
			this.diagnosticSeries = await client.request('sessions.getDiagnosticWindow', {
				sessionId: selected.sessionId,
				centerElapsedMs: Math.max(0, Math.round(centerElapsedMs)),
				beforeMs: 15_000,
				afterMs: 15_000,
				fields: [
					'packVoltageV',
					'dutyCycle',
					'controllerTempC',
					'motorTempC',
					'footpadState',
					'motorCurrentA',
					'batteryCurrentA',
					'erpm',
					'speedKph'
				]
			});
		});
	}

	async openBoardProfile(savedDeviceId: string): Promise<void> {
		await this.run(async (client) => {
			this.selectedBoardProfile = await client.request('boardProfiles.get', { savedDeviceId });
		});
	}

	async updateBoardProfile(patch: Record<string, unknown>): Promise<void> {
		const selected = this.selectedBoardProfile;
		if (!selected) return;
		await this.run(async (client) => {
			this.selectedBoardProfile = await client.request('boardProfiles.update', {
				savedDeviceId: selected.savedDeviceId,
				expectedRevision: selected.revision,
				patch
			});
			this.savedDevices = await client.request('savedDevices.list', {});
		});
	}

	closeBoardProfile(): void {
		this.selectedBoardProfile = null;
	}

	closeSession(): void {
		this.selectedSession = null;
		this.sessionSeries = [];
		this.diagnosticSeries = [];
		this.exportStatus = null;
	}

	close(): void {
		this.client?.close();
		this.client = null;
	}

	private async run(action: (client: ApiClient) => Promise<void>): Promise<void> {
		if (!this.client?.isOpen) await this.connect();
		if (!this.client?.isOpen) return;
		const client = this.client;
		this.busy = true;
		this.error = null;
		try {
			await action(client);
		} catch (error) {
			this.error = error instanceof Error ? error.message : String(error);
		} finally {
			this.busy = false;
		}
	}
}

export const apiStore = new ApiStore();
