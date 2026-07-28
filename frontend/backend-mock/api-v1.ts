import type {
	AppState,
	BackendError,
	BackendMethod,
	Request,
	Response,
	SeriesPoint,
	SessionSummary,
	StateChanged
} from '../src/lib/backend/contract.js';

const discoveredBoard = {
	deviceId: 'mock-board',
	name: 'Float View Test VESC',
	state: 'discovered' as const,
	lastSeenAt: '2026-01-01T00:00:00Z'
};

const savedBoard = {
	savedDeviceId: 'saved-board',
	deviceHistoryId: 'history-board',
	displayName: 'Float View Test VESC',
	advertisedName: 'Float View Test VESC',
	lastSeenAt: '2026-01-01T00:00:00Z',
	lastConnectedAt: null,
	profileComplete: false
};

const session: SessionSummary = {
	sessionId: 'session-1',
	deviceHistoryId: 'history-board',
	displayName: 'Float View Test VESC',
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

const series: SeriesPoint[] = [
	{ elapsedMs: 0, values: { packVoltageV: 60, dutyCycle: 0.1, speedKph: 0 } },
	{ elapsedMs: 60_000, values: { packVoltageV: 55, dutyCycle: 0.8, speedKph: 20 } }
];

export class MockBackend {
	readonly profiles = new Map([
		[
			savedBoard.savedDeviceId,
			{
				savedDeviceId: savedBoard.savedDeviceId,
				revision: 0,
				values: {} as Record<string, unknown>,
				missingFields: ['seriesCount', 'parallelCount', 'motor']
			}
		]
	]);
	private state: AppState = {
		revision: 0,
		bluetooth: {
			radio: 'powered_on',
			scanPermission: 'granted',
			connectPermission: 'granted',
			scanning: false
		},
		discoveredDevices: [],
		savedDevices: [savedBoard],
		activeConnection: null,
		recentSessions: [session],
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

	constructor(private readonly publish: (event: StateChanged) => void = () => {}) {}

	handlePayload(payload: string): Response {
		if (payload.length > 65_536) return failure(0, 'invalid_request', 'Message is too large');
		let value: unknown;
		try {
			value = JSON.parse(payload);
		} catch {
			return failure(0, 'invalid_request', 'Invalid JSON');
		}
		return this.handle(value);
	}

	handle(value: unknown): Response {
		if (!value || typeof value !== 'object') {
			return failure(0, 'invalid_request', 'Request must be an object');
		}
		const request = value as Partial<Request>;
		if (!Number.isSafeInteger(request.id) || typeof request.method !== 'string') {
			return failure(0, 'invalid_request', 'Request ID and method are required');
		}
		const id = request.id!;
		const params = asObject(request.params);
		if (!params) return failure(id, 'invalid_request', 'Params must be an object');

		switch (request.method as BackendMethod) {
			case 'state.get':
				return success(id, this.state);
			case 'bluetooth.requestPermission':
				return success(id, this.state.bluetooth);
			case 'bluetooth.scan':
				this.change({
					...this.state,
					bluetooth: { ...this.state.bluetooth, scanning: false },
					discoveredDevices: [discoveredBoard]
				});
				return accepted(id);
			case 'device.connect': {
				const deviceId = string(params.deviceId);
				if (!deviceId) return failure(id, 'invalid_request', 'deviceId is required');
				if (deviceId !== discoveredBoard.deviceId) {
					return failure(id, 'not_found', 'Device is no longer available');
				}
				if (this.state.activeConnection) {
					return failure(id, 'invalid_state', 'A board is already active');
				}
				this.change({
					...this.state,
					activeConnection: {
						...discoveredBoard,
						state: 'ready',
						firmware: {
							vescVersion: '6.06',
							packageName: 'refloat',
							packageVersion: '1.2.3',
							hardwareName: 'Express',
							supported: true
						}
					}
				});
				return accepted(id);
			}
			case 'device.disconnect':
				this.change({ ...this.state, activeConnection: null });
				return accepted(id);
			case 'settings.update': {
				const expectedRevision = number(params.expectedRevision);
				const patch = asObject(params.patch);
				if (expectedRevision === null || !patch) {
					return failure(id, 'invalid_request', 'Revision and patch are required');
				}
				if (expectedRevision !== this.state.settings.revision) {
					return failure(id, 'conflict', 'Settings revision changed');
				}
				const settings = {
					...this.state.settings,
					...patch,
					revision: this.state.settings.revision + 1
				};
				this.change({ ...this.state, settings });
				return success(id, settings);
			}
			case 'boardProfile.get': {
				const profile = this.profiles.get(string(params.savedDeviceId) ?? '');
				return profile
					? success(id, profile)
					: failure(id, 'not_found', 'Board Profile was not found');
			}
			case 'boardProfile.update': {
				const savedDeviceId = string(params.savedDeviceId);
				const expectedRevision = number(params.expectedRevision);
				const patch = asObject(params.patch);
				const current = savedDeviceId ? this.profiles.get(savedDeviceId) : undefined;
				if (!savedDeviceId || !current || expectedRevision === null || !patch) {
					return failure(id, 'invalid_request', 'Valid profile update parameters are required');
				}
				if (expectedRevision !== current.revision) {
					return failure(id, 'conflict', 'Board Profile revision changed');
				}
				const updated = {
					...current,
					revision: current.revision + 1,
					values: { ...current.values, ...patch },
					missingFields: current.missingFields.filter((field) => !(field in patch))
				};
				this.profiles.set(savedDeviceId, updated);
				this.change({
					...this.state,
					savedDevices: this.state.savedDevices.map((device) =>
						device.savedDeviceId === savedDeviceId
							? { ...device, profileComplete: updated.missingFields.length === 0 }
							: device
					)
				});
				return success(id, updated);
			}
			case 'session.get': {
				const found = this.findSession(params);
				return found
					? success(id, { summary: found, series })
					: failure(id, 'not_found', 'Session was not found');
			}
			case 'session.getDiagnostics':
				return this.findSession(params)
					? success(id, [
							{ elapsedMs: number(params.centerElapsedMs) ?? 0, values: { packVoltageV: 57 } }
						])
					: failure(id, 'not_found', 'Session was not found');
			case 'session.export':
				return this.findSession(params)
					? success(id, {
							fileName: `${string(params.sessionId)}.${params.format === 'csv' ? 'csv' : 'floatview'}`,
							delivery: 'staged'
						})
					: failure(id, 'not_found', 'Session was not found');
			case 'session.delete': {
				const found = this.findSession(params);
				if (!found) return failure(id, 'not_found', 'Session was not found');
				this.change({
					...this.state,
					recentSessions: this.state.recentSessions.filter(
						(item) => item.sessionId !== found.sessionId
					)
				});
				return accepted(id);
			}
			case 'sessionGroup.unlink': {
				const groupId = string(params.sessionGroupId);
				if (!this.state.sessionGroups.some((group) => group.sessionGroupId === groupId)) {
					return failure(id, 'not_found', 'Session group was not found');
				}
				this.change({
					...this.state,
					sessionGroups: this.state.sessionGroups.filter(
						(group) => group.sessionGroupId !== groupId
					)
				});
				return accepted(id);
			}
			default:
				return failure(id, 'invalid_request', 'Unknown method');
		}
	}

	private findSession(params: Record<string, unknown>) {
		const sessionId = string(params.sessionId);
		return this.state.recentSessions.find((item) => item.sessionId === sessionId);
	}

	private change(next: AppState) {
		this.state = { ...next, revision: this.state.revision + 1 };
		this.publish({ event: 'state.changed', data: this.state });
	}
}

function success(id: number, result: unknown): Response {
	return { id, result };
}

function accepted(id: number): Response {
	return success(id, { accepted: true });
}

function failure(id: number, code: BackendError['code'], message: string): Response {
	return { id, error: { code, message } };
}

function asObject(value: unknown): Record<string, unknown> | null {
	return value !== null && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: null;
}

function string(value: unknown): string | null {
	return typeof value === 'string' && value.length > 0 ? value : null;
}

function number(value: unknown): number | null {
	return typeof value === 'number' && Number.isFinite(value) ? value : null;
}
