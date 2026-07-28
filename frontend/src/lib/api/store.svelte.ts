import type {
	AppState,
	BluetoothState,
	BoardProfile,
	SeriesPoint,
	SessionSummary,
	Settings
} from '$lib/backend/contract';
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
	snapshot = $state<AppState | null>(null);
	error = $state<string | null>(null);
	selectedSession = $state<SessionSummary | null>(null);
	sessionSeries = $state<SeriesPoint[]>([]);
	diagnosticSeries = $state<SeriesPoint[]>([]);
	selectedBoardProfile = $state<BoardProfile | null>(null);
	exportStatus = $state<string | null>(null);
	busy = $state(false);
	private client: ApiClient | null = null;
	private unsubscribeState: (() => void) | null = null;

	constructor(
		private readonly bootstrapLoader = loadBootstrap,
		private readonly bootstrapConnector = connectBootstrap
	) {}

	get savedDevices() {
		return this.snapshot?.savedDevices ?? [];
	}

	get discoveredDevices() {
		return this.snapshot?.discoveredDevices ?? [];
	}

	get firmware() {
		return this.snapshot?.activeConnection?.firmware ?? null;
	}

	get sessions() {
		return this.snapshot?.recentSessions ?? [];
	}

	get sessionGroups() {
		return this.snapshot?.sessionGroups ?? [];
	}

	get settings() {
		return this.snapshot?.settings ?? null;
	}

	async connect(): Promise<void> {
		this.status = this.client ? 'reconnecting' : 'starting';
		this.unsubscribeState?.();
		this.client?.close();
		this.client = null;
		try {
			this.client = await this.bootstrapConnector(await this.bootstrapLoader());
			this.unsubscribeState = this.client.onStateChanged((state) => this.acceptState(state));
			this.acceptState(await this.client.request('state.get', {}));
			this.status = 'connected';
			this.error = null;
		} catch (error) {
			this.status = 'failed';
			this.error = message(error);
		}
	}

	async scan(): Promise<void> {
		await this.run(async (client) => {
			try {
				await client.request('bluetooth.scan', { timeoutMs: 5_000 });
			} catch (error) {
				if (!(error instanceof ApiRequestError) || error.code !== 'permission_required')
					throw error;
				const bluetooth = await client.request('bluetooth.requestPermission', {});
				if (permissionOutcome(bluetooth) !== 'ready') {
					throw new Error('Bluetooth permission is required before scanning.');
				}
				await client.request('bluetooth.scan', { timeoutMs: 5_000 });
			}
		});
	}

	async connectDevice(deviceId: string): Promise<void> {
		await this.run((client) =>
			client.request('device.connect', { deviceId }).then(() => undefined)
		);
	}

	async disconnectDevice(): Promise<void> {
		await this.run((client) => client.request('device.disconnect', {}).then(() => undefined));
	}

	async updateSessionSettings(patch: Omit<Partial<Settings>, 'revision'>): Promise<void> {
		const settings = this.settings;
		if (!settings) return;
		await this.run(async (client) => {
			const updated = await client.request('settings.update', {
				expectedRevision: settings.revision,
				patch
			});
			if (this.snapshot) this.acceptState({ ...this.snapshot, settings: updated });
		});
	}

	async openBoardProfile(savedDeviceId: string): Promise<void> {
		await this.run(async (client) => {
			this.selectedBoardProfile = await client.request('boardProfile.get', { savedDeviceId });
		});
	}

	async updateBoardProfile(patch: Record<string, unknown>): Promise<void> {
		const profile = this.selectedBoardProfile;
		if (!profile) return;
		await this.run(async (client) => {
			this.selectedBoardProfile = await client.request('boardProfile.update', {
				savedDeviceId: profile.savedDeviceId,
				expectedRevision: profile.revision,
				patch
			});
		});
	}

	async openSession(sessionId: string): Promise<void> {
		await this.run(async (client) => {
			const detail = await client.request('session.get', { sessionId, maxPoints: 400 });
			this.selectedSession = detail.summary;
			this.sessionSeries = detail.series;
			this.diagnosticSeries = [];
			this.exportStatus = null;
		});
	}

	async loadDiagnosticWindow(centerElapsedMs: number): Promise<void> {
		const session = this.selectedSession;
		if (!session) return;
		await this.run(async (client) => {
			this.diagnosticSeries = await client.request('session.getDiagnostics', {
				sessionId: session.sessionId,
				centerElapsedMs: Math.max(0, Math.round(centerElapsedMs)),
				beforeMs: 15_000,
				afterMs: 15_000
			});
		});
	}

	async exportSession(format: 'float-view-archive' | 'csv'): Promise<void> {
		const session = this.selectedSession;
		if (!session) return;
		await this.run(async (client) => {
			const result = await client.request('session.export', {
				sessionId: session.sessionId,
				format
			});
			this.exportStatus =
				result.delivery === 'presented'
					? `${result.fileName} is ready to save or share.`
					: `${result.fileName} is staged by the test backend.`;
		});
	}

	async deleteSelectedSession(): Promise<void> {
		const session = this.selectedSession;
		if (!session) return;
		await this.run(async (client) => {
			await client.request('session.delete', { sessionId: session.sessionId });
			this.closeSession();
		});
	}

	async unlinkSessionGroup(sessionGroupId: string): Promise<void> {
		await this.run((client) =>
			client.request('sessionGroup.unlink', { sessionGroupId }).then(() => undefined)
		);
	}

	closeBoardProfile() {
		this.selectedBoardProfile = null;
	}

	closeSession() {
		this.selectedSession = null;
		this.sessionSeries = [];
		this.diagnosticSeries = [];
		this.exportStatus = null;
	}

	close() {
		this.unsubscribeState?.();
		this.unsubscribeState = null;
		this.client?.close();
		this.client = null;
	}

	private acceptState(state: AppState) {
		if (this.snapshot && state.revision < this.snapshot.revision) return;
		this.snapshot = state;
	}

	private async run(action: (client: ApiClient) => Promise<void>) {
		if (!this.client?.isOpen) await this.connect();
		if (!this.client?.isOpen) return;
		this.busy = true;
		this.error = null;
		try {
			await action(this.client);
		} catch (error) {
			this.error = message(error);
		} finally {
			this.busy = false;
		}
	}
}

function message(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

export const apiStore = new ApiStore();
