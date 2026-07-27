import type {
	Device,
	HelloResult,
	SavedDevice,
	Session,
	Snapshot
} from '../../../../api/generated/typescript/models.js';
import { connectBootstrap, loadBootstrap } from './bootstrap';
import type { ApiClient } from './client';

export type ApiConnectionStatus = 'starting' | 'connected' | 'reconnecting' | 'failed';

export class ApiStore {
	status = $state<ApiConnectionStatus>('starting');
	snapshot = $state<Snapshot | null>(null);
	hello = $state<HelloResult | null>(null);
	error = $state<string | null>(null);
	savedDevices = $state<SavedDevice[]>([]);
	discoveredDevices = $state<Device[]>([]);
	sessions = $state<Session[]>([]);
	selectedSession = $state<Session | null>(null);
	busy = $state(false);
	private client: ApiClient | null = null;

	async connect(): Promise<void> {
		this.status = this.client ? 'reconnecting' : 'starting';
		try {
			const bootstrap = await loadBootstrap();
			this.client = await connectBootstrap(bootstrap);
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
		const [savedDevices, sessions, snapshot] = await Promise.all([
			this.client.request('savedDevices.list', {}),
			this.client.request('sessions.list', { limit: 50 }),
			this.client.request('system.getSnapshot', {})
		]);
		this.savedDevices = savedDevices;
		this.sessions = sessions.items;
		this.snapshot = snapshot;
		this.discoveredDevices = snapshot.knownDevices;
	}

	async scan(): Promise<void> {
		await this.run(async (client) => {
			await client.request('discovery.start', { timeoutMs: 5_000 });
			await this.refreshCollections();
		});
	}

	async connectDevice(deviceId: string): Promise<void> {
		await this.run(async (client) => {
			await client.request('device.connect', { deviceId, timeoutMs: 10_000 });
			await this.refreshCollections();
		});
	}

	async disconnectDevice(): Promise<void> {
		await this.run(async (client) => {
			await client.request('device.disconnect', {});
			await this.refreshCollections();
		});
	}

	async openSession(sessionId: string): Promise<void> {
		await this.run(async (client) => {
			this.selectedSession = await client.request('sessions.getSummary', { sessionId });
		});
	}

	closeSession(): void {
		this.selectedSession = null;
	}

	close(): void {
		this.client?.close();
		this.client = null;
	}

	private async run(action: (client: ApiClient) => Promise<void>): Promise<void> {
		if (!this.client) return;
		this.busy = true;
		this.error = null;
		try {
			await action(this.client);
		} catch (error) {
			this.error = error instanceof Error ? error.message : String(error);
		} finally {
			this.busy = false;
		}
	}
}

export const apiStore = new ApiStore();
