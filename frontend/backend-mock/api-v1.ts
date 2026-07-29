import type {
	AppState,
	BackendError,
	BackendMethod,
	ConnectionIssueCode,
	ControllerCandidate,
	Request,
	Response,
	SeriesPoint,
	SessionSummary,
	StateChanged
} from '../src/lib/backend/contract.ts';

const discoveredBoard = {
	deviceId: 'mock-board',
	name: 'Float View Test VESC',
	state: 'discovered' as const,
	lastSeenAt: '2026-01-01T00:00:00Z'
};

const savedBoard = {
	savedDeviceId: 'saved-board',
	deviceHistoryId: 'history-board',
	revision: 0,
	displayName: 'Float View Test VESC',
	customDisplayName: null,
	advertisedName: 'Float View Test VESC',
	bluetoothIdentifier: 'AA:BB:CC:DD:EE:FF',
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

const secondSession: SessionSummary = {
	...session,
	sessionId: 'session-2',
	startedAt: '2026-01-01T00:02:00Z',
	endedAt: '2026-01-01T00:03:00Z'
};

const series: SeriesPoint[] = [
	{ elapsedMs: 0, values: { packVoltageV: 60, dutyCycle: 0.1, speedKph: 0 } },
	{ elapsedMs: 60_000, values: { packVoltageV: 55, dutyCycle: 0.8, speedKph: 20 } }
];

export class MockBackend {
	private readonly publish: (event: StateChanged) => void;
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
	private nextSession = 3;
	private activeSessionId?: string;
	private pendingCandidates: ControllerCandidate[] = [];
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
		recentSessions: [secondSession, session],
		sessionGroups: [
			{
				sessionGroupId: 'group-1',
				deviceHistoryId: 'history-board',
				sessionIds: [session.sessionId, secondSession.sessionId],
				totalElapsedMs: session.totalElapsedMs + secondSession.totalElapsedMs,
				distanceM: (session.distanceM ?? 0) + (secondSession.distanceM ?? 0)
			}
		],
		settings: {
			revision: 0,
			idleProbeIntervalMs: 500,
			idlePersistIntervalMs: 1_000,
			ridingPersistIntervalMs: 100,
			postRideTailMs: 15_000,
			reconnectDurationMs: 300_000,
			automaticSessionLinkingEnabled: false,
			sessionLinkWindowMs: 7_200_000,
			themePreference: 'system',
			unitPreference: 'metric'
		}
	};

	constructor(publish: (event: StateChanged) => void = () => {}) {
		this.publish = publish;
	}

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
				const active = this.state.activeConnection;
				if (active && (active.deviceId !== deviceId || active.state !== 'failed')) {
					return failure(id, 'invalid_state', 'A board is already active');
				}
				if (!active && deviceId !== discoveredBoard.deviceId) {
					return failure(id, 'not_found', 'Device is no longer available');
				}
				this.activeSessionId = `session-${this.nextSession++}`;
				this.change({
					...this.state,
					activeConnection: {
						...discoveredBoard,
						state: 'ready',
						savedDeviceId: savedBoard.savedDeviceId,
						displayName: savedBoard.displayName,
						controllerConnection: { kind: 'direct' },
						controllerCandidates: [],
						connectionIssue: null,
						activeSession: {
							...session,
							sessionId: this.activeSessionId,
							startedAt: new Date().toISOString(),
							endedAt: null,
							state: 'active' as const,
							sampleCount: 0,
							ridingElapsedMs: 0,
							totalElapsedMs: 0
						},
						latestSample: null,
						firmware: {
							vescVersion: '6.06',
							packageName: 'refloat',
							packageVersion: '1.2.3',
							hardwareName: 'Express',
							supported: true
						}
					},
					recentSessions: [
						{
							...session,
							sessionId: this.activeSessionId,
							startedAt: new Date().toISOString(),
							endedAt: null,
							state: 'active' as const,
							sampleCount: 0,
							ridingElapsedMs: 0,
							totalElapsedMs: 0
						},
						...this.state.recentSessions
					].slice(0, 50)
				});
				return accepted(id);
			}
			case 'device.selectController': {
				const deviceId = string(params.deviceId);
				const canId = integer(params.canId);
				if (!deviceId || canId === null || canId < 0 || canId > 253) {
					return failure(id, 'invalid_request', 'Valid deviceId and CAN ID are required');
				}
				const active = this.state.activeConnection;
				const candidate = this.pendingCandidates.find((value) => value.canId === canId);
				if (
					!active ||
					active.deviceId !== deviceId ||
					active.state !== 'selecting_controller' ||
					!candidate?.vescSupported
				) {
					return failure(id, 'invalid_state', 'Controller selection is no longer available');
				}
				const profile = this.profiles.get(active.savedDeviceId ?? '');
				if (!profile) return failure(id, 'invalid_state', 'Saved board is unavailable');
				const updated = {
					...profile,
					revision: profile.revision + 1,
					values: { ...profile.values, preferredCanId: canId }
				};
				this.profiles.set(profile.savedDeviceId, updated);
				this.pendingCandidates = [];
				this.change({
					...this.state,
					activeConnection: {
						...active,
						state: 'identifying',
						controllerConnection: { kind: 'can', canId },
						controllerCandidates: [],
						connectionIssue: null
					}
				});
				return accepted(id);
			}
			case 'savedDevice.update': {
				const savedDeviceId = string(params.savedDeviceId);
				const expectedRevision = number(params.expectedRevision);
				const current = this.state.savedDevices.find(
					(device) => device.savedDeviceId === savedDeviceId
				);
				const name = params.customDisplayName;
				if (!current || expectedRevision === null || (name !== null && typeof name !== 'string')) {
					return failure(
						id,
						'invalid_request',
						'Valid saved device update parameters are required'
					);
				}
				if (current.revision !== expectedRevision) {
					return failure(id, 'conflict', 'Saved device revision changed');
				}
				const customDisplayName = typeof name === 'string' && name.trim() ? name.trim() : null;
				const updated = {
					...current,
					revision: current.revision + 1,
					customDisplayName,
					displayName: customDisplayName ?? current.advertisedName ?? current.bluetoothIdentifier
				};
				this.change({
					...this.state,
					savedDevices: this.state.savedDevices.map((device) =>
						device.savedDeviceId === savedDeviceId ? updated : device
					),
					activeConnection:
						this.state.activeConnection?.savedDeviceId === savedDeviceId
							? {
									...this.state.activeConnection,
									name: updated.displayName,
									displayName: updated.displayName
								}
							: this.state.activeConnection
				});
				return success(id, updated);
			}
			case 'device.disconnect': {
				const activeSessionId = this.activeSessionId;
				this.activeSessionId = undefined;
				this.change({
					...this.state,
					activeConnection: null,
					recentSessions: this.state.recentSessions.map((item) =>
						item.sessionId === activeSessionId
							? { ...item, endedAt: new Date().toISOString(), state: 'completed' }
							: item
					)
				});
				return accepted(id);
			}
			case 'settings.update': {
				const expectedRevision = number(params.expectedRevision);
				const patch = asObject(params.patch);
				if (expectedRevision === null || !patch) {
					return failure(id, 'invalid_request', 'Revision and patch are required');
				}
				if (expectedRevision !== this.state.settings.revision) {
					return failure(id, 'conflict', 'Settings revision changed');
				}
				const allowed = new Set([
					'idleProbeIntervalMs',
					'idlePersistIntervalMs',
					'ridingPersistIntervalMs',
					'postRideTailMs',
					'reconnectDurationMs',
					'automaticSessionLinkingEnabled',
					'sessionLinkWindowMs',
					'themePreference',
					'unitPreference'
				]);
				if (
					!Object.keys(patch).length ||
					Object.keys(patch).some((key) => !allowed.has(key)) ||
					('themePreference' in patch &&
						!['system', 'light', 'dark'].includes(String(patch.themePreference))) ||
					('unitPreference' in patch &&
						!['metric', 'imperial'].includes(String(patch.unitPreference)))
				) {
					return failure(id, 'invalid_request', 'Settings patch is invalid');
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
				if (
					'preferredCanId' in patch &&
					patch.preferredCanId !== null &&
					(integer(patch.preferredCanId) === null ||
						(integer(patch.preferredCanId) ?? -1) < 0 ||
						(integer(patch.preferredCanId) ?? 254) > 253)
				) {
					return failure(id, 'invalid_request', 'Preferred CAN ID must be 0 through 253');
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
				if (!found) return accepted(id);
				this.change({
					...this.state,
					recentSessions: this.state.recentSessions.filter(
						(item) => item.sessionId !== found.sessionId
					),
					sessionGroups: this.state.sessionGroups
						.map((group) => ({
							...group,
							sessionIds: group.sessionIds.filter((id) => id !== found.sessionId)
						}))
						.filter((group) => group.sessionIds.length >= 2)
				});
				return accepted(id);
			}
			case 'sessionGroup.unlink': {
				const groupId = string(params.sessionGroupId);
				if (!this.state.sessionGroups.some((group) => group.sessionGroupId === groupId)) {
					return accepted(id);
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

	publishTelemetry(): void {
		const active = this.state.activeConnection;
		if (!active || active.state !== 'ready' || !active.activeSession) return;
		const sequence = (active.latestSample?.sequence ?? 0) + 1;
		const elapsedMs = sequence * this.state.settings.ridingPersistIntervalMs;
		const latestSample = {
			sequence,
			observedAt: new Date().toISOString(),
			elapsedMs,
			rideState: 'riding' as const,
			boardState: 1,
			footpadState: 3,
			packVoltageV: 58.4 - sequence * 0.002,
			dutyCycle: 0.42,
			controllerTempC: 38.2,
			motorTempC: 41.7,
			motorCurrentA: 18.6,
			batteryCurrentA: 8.4,
			erpm: 3240,
			speedKph: 24.3
		};
		const activeSession = {
			...active.activeSession,
			sampleCount: sequence,
			ridingElapsedMs: elapsedMs,
			totalElapsedMs: elapsedMs,
			maxSpeedMps: latestSample.speedKph / 3.6,
			maxDutyCycle: latestSample.dutyCycle
		};
		this.change({
			...this.state,
			activeConnection: { ...active, latestSample, activeSession },
			recentSessions: this.state.recentSessions.map((item) =>
				item.sessionId === activeSession.sessionId ? activeSession : item
			)
		});
	}

	get telemetryCadenceMs(): number {
		return this.state.settings.ridingPersistIntervalMs;
	}

	simulateConnectionState(state: 'ready' | 'reconnecting'): void {
		if (!this.state.activeConnection) return;
		this.change({
			...this.state,
			activeConnection: { ...this.state.activeConnection, state }
		});
	}

	simulateIdentifying(): void {
		const active = this.state.activeConnection;
		if (!active) return;
		const activeSessionId = active.activeSession?.sessionId;
		this.activeSessionId = undefined;
		this.change({
			...this.state,
			activeConnection: {
				...active,
				state: 'identifying',
				savedDeviceId: savedBoard.savedDeviceId,
				controllerConnection: null,
				controllerCandidates: [],
				connectionIssue: null,
				activeSession: null,
				firmware: null
			},
			recentSessions: activeSessionId
				? this.state.recentSessions.filter((item) => item.sessionId !== activeSessionId)
				: this.state.recentSessions
		});
	}

	simulateCanReady(canId: number): void {
		const active = this.state.activeConnection;
		if (!active || !Number.isInteger(canId) || canId < 0 || canId > 253) return;
		this.change({
			...this.state,
			activeConnection: {
				...active,
				state: 'ready',
				controllerConnection: { kind: 'can', canId },
				controllerCandidates: [],
				connectionIssue: null
			}
		});
	}

	simulateControllerSelection(candidates: ControllerCandidate[]): void {
		const active = this.state.activeConnection;
		if (!active || candidates.length < 2 || candidates.length > 16) return;
		this.pendingCandidates = [...candidates].sort((left, right) => left.canId - right.canId);
		this.change({
			...this.state,
			activeConnection: {
				...active,
				state: 'selecting_controller',
				controllerConnection: null,
				controllerCandidates: this.pendingCandidates,
				connectionIssue: null,
				activeSession: null
			}
		});
	}

	simulateConnectionFailure(code: ConnectionIssueCode, message: string): void {
		const active = this.state.activeConnection;
		if (!active) return;
		const activeSessionId = active.activeSession?.sessionId;
		this.activeSessionId = undefined;
		this.change({
			...this.state,
			activeConnection: {
				...active,
				state: 'failed',
				connectionIssue: { code, message: message.slice(0, 1_024) },
				controllerCandidates: [],
				activeSession: null
			},
			recentSessions: activeSessionId
				? code === 'controller_changed'
					? this.state.recentSessions.map((item) =>
							item.sessionId === activeSessionId
								? {
										...item,
										endedAt: new Date().toISOString(),
										state: 'interrupted' as const,
										integrity: 'gaps' as const
									}
								: item
						)
					: this.state.recentSessions.filter((item) => item.sessionId !== activeSessionId)
				: this.state.recentSessions
		});
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

function integer(value: unknown): number | null {
	return typeof value === 'number' && Number.isInteger(value) ? value : null;
}
