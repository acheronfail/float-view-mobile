import type {
	AlertRule,
	BoardProfile,
	Device,
	HelloParams,
	HelloResult,
	JsonRpcId,
	LoggingJob,
	RpcErrorResponse,
	SavedDevice,
	Session,
	SessionGroup,
	Settings,
	Snapshot,
	StorageState
} from '../../api/generated/typescript/models.js';

type JsonRpcRequest = {
	jsonrpc?: unknown;
	id?: unknown;
	method?: unknown;
	params?: unknown;
};

type MockState = {
	revision: number;
	knownDevices: Device[];
	activeConnection: Device | null;
	savedDevices: SavedDevice[];
	profiles: Map<string, BoardProfile>;
	sessions: Session[];
	groups: SessionGroup[];
	settings: Settings;
	rules: AlertRule[];
	loggingJobs: LoggingJob[];
};

export type ApiV1Session = {
	initialized: boolean;
	state?: MockState;
};

export type ApiV1Response = { jsonrpc: '2.0'; id: JsonRpcId; result: unknown } | RpcErrorResponse;

const bluetoothState = {
	radio: 'powered_on' as const,
	scanPermission: 'granted' as const,
	connectPermission: 'granted' as const
};

function createState(): MockState {
	return {
		revision: 0,
		knownDevices: [],
		activeConnection: null,
		savedDevices: [],
		profiles: new Map(),
		sessions: [],
		groups: [],
		settings: {
			revision: 0,
			idleProbeIntervalMs: 500,
			idlePersistIntervalMs: 1000,
			ridingPersistIntervalMs: 100,
			postRideTailMs: 15_000,
			reconnectDurationMs: 300_000,
			automaticSessionLinkingEnabled: false,
			sessionLinkWindowMs: 7_200_000,
			rawPacketCaptureEnabled: false
		},
		rules: [],
		loggingJobs: []
	};
}

function stateFor(session: ApiV1Session): MockState {
	return (session.state ??= createState());
}

function combinedDistance(
	first: number | null | undefined,
	second: number | null | undefined
): number | null {
	return typeof first === 'number' && typeof second === 'number' ? first + second : null;
}

function snapshot(state: MockState): Snapshot {
	return {
		revision: state.revision,
		bluetooth: bluetoothState,
		knownDevices: state.knownDevices,
		activeConnection: state.activeConnection,
		telemetrySubscriptions: [],
		loggingJobs: state.loggingJobs
	};
}

function helloResult(state: MockState): HelloResult {
	return {
		api: { major: 1, minor: 0 },
		backend: {
			name: 'float-view-mock',
			version: '0.1.0',
			platform: 'mock'
		},
		capabilities: [
			{ name: 'system.snapshot', available: true },
			{ name: 'device.connection', available: true },
			{ name: 'logging.durable', available: true },
			{ name: 'background.mode', available: true, constraints: { behavior: 'deterministic_fake' } }
		],
		limits: {
			minTelemetryIntervalMs: 50,
			maxMessageBytes: 65_536,
			maxReconnectDurationMs: 3_600_000
		},
		snapshot: snapshot(state)
	};
}

function error(
	id: JsonRpcId | null,
	code: number,
	kind:
		| 'invalid_state'
		| 'unsupported'
		| 'protocol_error'
		| 'device_not_found'
		| 'storage_unavailable',
	message: string,
	recoverable = false
): RpcErrorResponse {
	return {
		jsonrpc: '2.0',
		id,
		error: {
			code,
			message,
			data: { kind, recoverable }
		}
	};
}

function isId(value: unknown): value is JsonRpcId {
	return (
		(typeof value === 'string' && value.length > 0 && value.length <= 128) ||
		(typeof value === 'number' && Number.isInteger(value))
	);
}

function record(value: unknown): Record<string, unknown> {
	return value && typeof value === 'object' ? (value as Record<string, unknown>) : {};
}

function isHelloParams(value: unknown): value is HelloParams {
	const params = record(value);
	const client = record(params.client);
	const api = record(params.api);
	return (
		typeof client.name === 'string' &&
		client.name.length > 0 &&
		typeof client.version === 'string' &&
		client.version.length > 0 &&
		typeof api.major === 'number' &&
		Number.isInteger(api.major) &&
		typeof api.minMinor === 'number' &&
		Number.isInteger(api.minMinor) &&
		typeof api.maxMinor === 'number' &&
		Number.isInteger(api.maxMinor) &&
		api.maxMinor >= api.minMinor
	);
}

function ok(id: JsonRpcId, result: unknown): ApiV1Response {
	return { jsonrpc: '2.0', id, result };
}

function requireString(params: Record<string, unknown>, name: string): string | null {
	return typeof params[name] === 'string' && params[name].length > 0
		? (params[name] as string)
		: null;
}

function storageState(availableBytes = 8 * 1024 ** 3): StorageState {
	return {
		availableBytes,
		level:
			availableBytes < 250 * 1024 ** 2
				? 'stop'
				: availableBytes < 500 * 1024 ** 2
					? 'critical'
					: availableBytes < 1024 ** 3
						? 'warning'
						: 'normal',
		warningBytes: 1024 ** 3,
		criticalBytes: 500 * 1024 ** 2,
		stopBytes: 250 * 1024 ** 2
	};
}

export function handleApiV1Request(session: ApiV1Session, input: unknown): ApiV1Response {
	if (!input || typeof input !== 'object') {
		return error(null, -32600, 'protocol_error', 'Invalid JSON-RPC request');
	}

	const request = input as JsonRpcRequest;
	const id = isId(request.id) ? request.id : null;
	if (request.jsonrpc !== '2.0' || id === null || typeof request.method !== 'string') {
		return error(id, -32600, 'protocol_error', 'Invalid JSON-RPC request');
	}
	const state = stateFor(session);

	if (request.method === 'system.hello') {
		if (!isHelloParams(request.params)) {
			return error(id, -32602, 'protocol_error', 'Invalid system.hello parameters');
		}
		if (request.params.api.major !== 1 || request.params.api.minMinor > 0) {
			return error(id, -32001, 'unsupported', 'Unsupported API version');
		}
		session.initialized = true;
		return ok(id, helloResult(state));
	}

	if (!session.initialized) {
		return error(
			id,
			-32000,
			'invalid_state',
			'system.hello must complete before other requests',
			true
		);
	}

	const params = record(request.params);
	switch (request.method) {
		case 'system.getSnapshot':
			return ok(id, snapshot(state));
		case 'bluetooth.getState':
		case 'bluetooth.requestPermission':
			return ok(id, bluetoothState);
		case 'discovery.start': {
			const devices: Device[] = [
				{ deviceId: 'fixture-thor300', name: 'Thor300', state: 'discovered' },
				{ deviceId: 'fixture-xrv', name: 'Floatwheel XRV', state: 'discovered' }
			];
			state.knownDevices = devices;
			state.revision += 1;
			return ok(id, { scanId: 'scan-1', devices });
		}
		case 'discovery.stop':
			return ok(id, { accepted: true });
		case 'device.connect': {
			const deviceId = requireString(params, 'deviceId');
			if (!deviceId) return error(id, -32602, 'protocol_error', 'deviceId is required');
			if (state.activeConnection && state.activeConnection.deviceId !== deviceId) {
				return error(id, -32000, 'invalid_state', 'Another board is active');
			}
			const discovered =
				state.knownDevices.find((device) => device.deviceId === deviceId) ??
				({ deviceId, name: 'Fixture board', state: 'discovered' } satisfies Device);
			state.activeConnection = { ...discovered, state: 'ready' };
			const saved: SavedDevice = {
				savedDeviceId: `saved-${deviceId}`,
				deviceHistoryId: `history-${deviceId}`,
				displayName: discovered.name,
				advertisedName: discovered.name,
				lastConnectedAt: new Date(0).toISOString(),
				profileComplete: false
			};
			if (!state.savedDevices.some((item) => item.savedDeviceId === saved.savedDeviceId)) {
				state.savedDevices.push(saved);
				state.profiles.set(saved.savedDeviceId, {
					savedDeviceId: saved.savedDeviceId,
					revision: 0,
					values: {},
					missingFields: ['seriesCount', 'parallelCount', 'motor']
				});
			}
			const sessionId = `session-${state.sessions.length + 1}`;
			const startedAtMs = state.sessions.length * 120_000;
			state.sessions.unshift({
				sessionId,
				deviceHistoryId: saved.deviceHistoryId,
				displayName: saved.displayName,
				startedAt: new Date(startedAtMs).toISOString(),
				endedAt: null,
				state: 'active',
				integrity: 'complete',
				sampleCount: 0,
				eventCount: 1,
				ridingElapsedMs: 0,
				totalElapsedMs: 0,
				distanceM: 0
			});
			state.loggingJobs = [
				{
					jobId: `job-${sessionId}`,
					deviceId,
					state: 'running',
					requestedIntervalMs: 1000,
					effectiveIntervalMs: 1000,
					startedAt: new Date(0).toISOString(),
					sampleCount: 0,
					errorCount: 0,
					bytesWritten: 0,
					backgroundCapable: true
				}
			];
			state.revision += 1;
			return ok(id, state.activeConnection);
		}
		case 'device.disconnect': {
			if (state.sessions[0]?.state === 'active') {
				const endedAtMs = Date.parse(state.sessions[0].startedAt) + 60_000;
				state.sessions[0] = {
					...state.sessions[0],
					state: 'completed',
					endedAt: new Date(endedAtMs).toISOString(),
					totalElapsedMs: 60_000,
					ridingElapsedMs: 45_000,
					sampleCount: 465,
					eventCount: 4,
					distanceM: 750
				};
				const current = state.sessions[0];
				const previous = state.sessions
					.slice(1)
					.find(
						(session) =>
							session.deviceHistoryId === current.deviceHistoryId &&
							typeof session.endedAt === 'string' &&
							Date.parse(session.endedAt) <= Date.parse(current.startedAt)
					);
				if (
					state.settings.automaticSessionLinkingEnabled &&
					previous?.endedAt &&
					Date.parse(current.startedAt) - Date.parse(previous.endedAt) <=
						state.settings.sessionLinkWindowMs
				) {
					const existing = state.groups.find((group) =>
						group.sessionIds.includes(previous.sessionId)
					);
					if (existing) {
						existing.sessionIds.push(current.sessionId);
						existing.totalElapsedMs += current.totalElapsedMs;
						existing.distanceM = combinedDistance(existing.distanceM, current.distanceM);
					} else {
						state.groups.push({
							sessionGroupId: `group-${state.groups.length + 1}`,
							deviceHistoryId: current.deviceHistoryId,
							sessionIds: [previous.sessionId, current.sessionId],
							totalElapsedMs: previous.totalElapsedMs + current.totalElapsedMs,
							distanceM: combinedDistance(previous.distanceM, current.distanceM)
						});
					}
				}
			}
			state.loggingJobs = [];
			state.activeConnection = null;
			state.revision += 1;
			return ok(id, { accepted: true });
		}
		case 'device.getActive':
			return ok(id, state.activeConnection);
		case 'savedDevices.list':
			return ok(id, state.savedDevices);
		case 'savedDevices.get': {
			const savedDeviceId = requireString(params, 'savedDeviceId');
			const value = state.savedDevices.find((item) => item.savedDeviceId === savedDeviceId);
			return value
				? ok(id, value)
				: error(id, -32004, 'device_not_found', 'Saved device not found');
		}
		case 'savedDevices.rename': {
			const savedDeviceId = requireString(params, 'savedDeviceId');
			const name = requireString(params, 'name');
			const index = state.savedDevices.findIndex((item) => item.savedDeviceId === savedDeviceId);
			if (index < 0 || !name)
				return error(id, -32004, 'device_not_found', 'Saved device not found');
			state.savedDevices[index] = { ...state.savedDevices[index], displayName: name };
			state.revision += 1;
			return ok(id, state.savedDevices[index]);
		}
		case 'savedDevices.forget': {
			const savedDeviceId = requireString(params, 'savedDeviceId');
			state.savedDevices = state.savedDevices.filter(
				(item) => item.savedDeviceId !== savedDeviceId
			);
			state.profiles.delete(savedDeviceId ?? '');
			state.revision += 1;
			return ok(id, { accepted: true });
		}
		case 'boardProfiles.get': {
			const value = state.profiles.get(requireString(params, 'savedDeviceId') ?? '');
			return value ? ok(id, value) : error(id, -32004, 'device_not_found', 'Profile not found');
		}
		case 'boardProfiles.update': {
			const savedDeviceId = requireString(params, 'savedDeviceId') ?? '';
			const value = state.profiles.get(savedDeviceId);
			if (!value) return error(id, -32004, 'device_not_found', 'Profile not found');
			if (params.expectedRevision !== value.revision) {
				return error(id, -32000, 'invalid_state', 'Profile revision conflict', true);
			}
			const patch = record(params.patch);
			const merged = { ...value.values, ...patch };
			const integerInRange = (key: string, min: number, max: number) =>
				merged[key] === undefined ||
				merged[key] === null ||
				(typeof merged[key] === 'number' &&
					Number.isInteger(merged[key]) &&
					merged[key] >= min &&
					merged[key] <= max);
			const validMotor =
				merged.motor === undefined ||
				merged.motor === null ||
				(typeof merged.motor === 'string' &&
					merged.motor.length > 0 &&
					new TextEncoder().encode(merged.motor).length <= 128);
			if (
				!integerInRange('seriesCount', 1, 36) ||
				!integerInRange('parallelCount', 1, 20) ||
				!validMotor
			) {
				return error(id, -32602, 'protocol_error', 'Invalid Board Profile patch');
			}
			const updated: BoardProfile = {
				...value,
				revision: value.revision + 1,
				values: merged,
				missingFields: value.missingFields.filter((field) => !(field in patch))
			};
			state.profiles.set(savedDeviceId, updated);
			const savedIndex = state.savedDevices.findIndex(
				(item) => item.savedDeviceId === savedDeviceId
			);
			if (savedIndex >= 0) {
				state.savedDevices[savedIndex] = {
					...state.savedDevices[savedIndex],
					profileComplete: updated.missingFields.length === 0
				};
			}
			return ok(id, updated);
		}
		case 'settings.get':
			return ok(id, state.settings);
		case 'settings.update': {
			if (params.expectedRevision !== state.settings.revision) {
				return error(id, -32000, 'invalid_state', 'Settings revision conflict', true);
			}
			const patch = record(params.patch);
			const allowed = new Set<keyof Settings>([
				'idleProbeIntervalMs',
				'idlePersistIntervalMs',
				'ridingPersistIntervalMs',
				'postRideTailMs',
				'reconnectDurationMs',
				'automaticSessionLinkingEnabled',
				'sessionLinkWindowMs',
				'rawPacketCaptureEnabled'
			]);
			if (
				Object.keys(patch).length === 0 ||
				Object.keys(patch).some((name) => !allowed.has(name as keyof Settings)) ||
				['automaticSessionLinkingEnabled', 'rawPacketCaptureEnabled'].some(
					(name) => name in patch && typeof patch[name] !== 'boolean'
				)
			) {
				return error(id, -32602, 'protocol_error', 'Invalid Settings patch', false);
			}
			const number = (name: keyof Settings, fallback: number, min: number, max: number) => {
				const value = patch[name];
				if (value !== undefined && (typeof value !== 'number' || !Number.isInteger(value))) {
					throw new TypeError('Invalid Settings patch');
				}
				return typeof value === 'number'
					? Math.max(min, Math.min(max, Math.round(value)))
					: fallback;
			};
			try {
				state.settings = {
					...state.settings,
					...(typeof patch.automaticSessionLinkingEnabled === 'boolean'
						? { automaticSessionLinkingEnabled: patch.automaticSessionLinkingEnabled }
						: {}),
					...(typeof patch.rawPacketCaptureEnabled === 'boolean'
						? { rawPacketCaptureEnabled: patch.rawPacketCaptureEnabled }
						: {}),
					revision: state.settings.revision + 1,
					idleProbeIntervalMs: number(
						'idleProbeIntervalMs',
						state.settings.idleProbeIntervalMs,
						100,
						10_000
					),
					idlePersistIntervalMs: number(
						'idlePersistIntervalMs',
						state.settings.idlePersistIntervalMs,
						100,
						60_000
					),
					ridingPersistIntervalMs: number(
						'ridingPersistIntervalMs',
						state.settings.ridingPersistIntervalMs,
						50,
						10_000
					),
					postRideTailMs: number('postRideTailMs', state.settings.postRideTailMs, 0, 300_000),
					reconnectDurationMs: number(
						'reconnectDurationMs',
						state.settings.reconnectDurationMs,
						0,
						3_600_000
					),
					sessionLinkWindowMs: number(
						'sessionLinkWindowMs',
						state.settings.sessionLinkWindowMs,
						60_000,
						86_400_000
					)
				};
			} catch {
				return error(id, -32602, 'protocol_error', 'Invalid Settings patch', false);
			}
			return ok(id, state.settings);
		}
		case 'sessions.list': {
			const limit = Math.max(1, Math.min(100, Number(params.limit ?? 20)));
			const filters = record(params.filters);
			const sort = record(params.sort);
			const field = String(sort.field ?? 'startedAt') as
				| 'startedAt'
				| 'totalElapsedMs'
				| 'ridingElapsedMs'
				| 'distanceM';
			const descending = String(sort.direction ?? 'desc') !== 'asc';
			const value = (session: Session): number =>
				field === 'startedAt'
					? Date.parse(session.startedAt)
					: field === 'distanceM'
						? (session.distanceM ?? -1)
						: Number(session[field]);
			const cursor = params.cursor
				? (JSON.parse(Buffer.from(String(params.cursor), 'base64url').toString()) as {
						field: string;
						value: number;
						sessionId: string;
					})
				: null;
			let candidates = state.sessions.filter(
				(session) =>
					(!filters.deviceHistoryId ||
						session.deviceHistoryId === String(filters.deviceHistoryId)) &&
					(!filters.state || session.state === String(filters.state)) &&
					(!filters.integrity || session.integrity === String(filters.integrity)) &&
					(filters.minimumTotalElapsedMs === undefined ||
						session.totalElapsedMs >= Number(filters.minimumTotalElapsedMs)) &&
					(filters.maximumTotalElapsedMs === undefined ||
						session.totalElapsedMs <= Number(filters.maximumTotalElapsedMs)) &&
					(filters.minimumRidingElapsedMs === undefined ||
						session.ridingElapsedMs >= Number(filters.minimumRidingElapsedMs)) &&
					(filters.maximumRidingElapsedMs === undefined ||
						session.ridingElapsedMs <= Number(filters.maximumRidingElapsedMs)) &&
					(filters.minimumDistanceM === undefined ||
						(session.distanceM ?? -1) >= Number(filters.minimumDistanceM)) &&
					(filters.maximumDistanceM === undefined ||
						(session.distanceM ?? -1) <= Number(filters.maximumDistanceM)) &&
					(!filters.startedAfter ||
						Date.parse(session.startedAt) >= Date.parse(String(filters.startedAfter))) &&
					(!filters.startedBefore ||
						Date.parse(session.startedAt) <= Date.parse(String(filters.startedBefore)))
			);
			candidates.sort((left, right) => {
				const comparison =
					value(left) - value(right) || left.sessionId.localeCompare(right.sessionId);
				return descending ? -comparison : comparison;
			});
			if (cursor) {
				if (cursor.field !== field) {
					return error(id, -32602, 'protocol_error', 'Cursor does not match sort field');
				}
				candidates = candidates.filter((session) => {
					const comparison =
						value(session) - cursor.value || session.sessionId.localeCompare(cursor.sessionId);
					return descending ? comparison < 0 : comparison > 0;
				});
			}
			const items = candidates.slice(0, limit);
			return ok(id, {
				items,
				nextCursor:
					items.length < candidates.length && items.length
						? Buffer.from(
								JSON.stringify({
									field,
									value: value(items.at(-1)!),
									sessionId: items.at(-1)!.sessionId
								})
							).toString('base64url')
						: null
			});
		}
		case 'sessions.get':
		case 'sessions.getSummary': {
			const value = state.sessions.find(
				(sessionItem) => sessionItem.sessionId === params.sessionId
			);
			return value ? ok(id, value) : error(id, -32004, 'device_not_found', 'Session not found');
		}
		case 'sessions.querySeries':
		case 'sessions.getDiagnosticWindow': {
			const start =
				request.method === 'sessions.querySeries'
					? Number(params.startElapsedMs)
					: Math.max(0, Number(params.centerElapsedMs) - Number(params.beforeMs));
			const end =
				request.method === 'sessions.querySeries'
					? Number(params.endElapsedMs)
					: Number(params.centerElapsedMs) + Number(params.afterMs);
			const fields = Array.isArray(params.fields) ? params.fields.map(String) : [];
			const requestedPoints =
				request.method === 'sessions.querySeries'
					? Math.max(1, Math.min(10_000, Number(params.maxPoints)))
					: 10_000;
			const interval =
				requestedPoints === 1
					? Math.max(1, end - start + 1)
					: Math.max(1, Math.ceil((end - start) / (requestedPoints - 1)));
			const points = [];
			for (let elapsedMs = start; elapsedMs <= end; elapsedMs += interval) {
				points.push({
					elapsedMs,
					values: Object.fromEntries(
						fields.map((field) => [
							field,
							field === 'packVoltageV'
								? 58 - elapsedMs / 3_600_000
								: field === 'dutyCycle'
									? 0.3 + 0.2 * Math.sin(elapsedMs / 2000)
									: 0
						])
					)
				});
			}
			return ok(id, points);
		}
		case 'sessions.listEvents':
			return ok(id, [
				{
					eventId: 'event-ride-start',
					elapsedMs: 5_000,
					kind: 'ride_state',
					details: { state: 'riding' }
				},
				{ eventId: 'event-footpad', elapsedMs: 30_000, kind: 'footpad', details: { state: 1 } }
			]);
		case 'sessions.export':
			if (state.sessions.find((item) => item.sessionId === params.sessionId)?.state === 'active') {
				return error(id, -32000, 'invalid_state', 'Active sessions cannot be exported');
			}
			if (!state.sessions.some((item) => item.sessionId === params.sessionId)) {
				return error(id, -32004, 'device_not_found', 'Session not found');
			}
			return ok(id, {
				exportId: `export-${String(params.sessionId)}`,
				fileName: `${String(params.sessionId)}.${params.format === 'csv' ? 'csv' : 'floatview'}`
			});
		case 'sessions.delete': {
			if (state.sessions.find((item) => item.sessionId === params.sessionId)?.state === 'active') {
				return error(id, -32000, 'invalid_state', 'Active sessions cannot be deleted');
			}
			state.sessions = state.sessions.filter((item) => item.sessionId !== params.sessionId);
			state.groups = state.groups
				.map((group) => ({
					...group,
					sessionIds: group.sessionIds.filter((sessionId) => sessionId !== params.sessionId)
				}))
				.filter((group) => group.sessionIds.length >= 2);
			return ok(id, { accepted: true });
		}
		case 'sessionGroups.list':
			return ok(id, state.groups);
		case 'sessionGroups.get': {
			const group = state.groups.find((item) => item.sessionGroupId === params.sessionGroupId);
			return group
				? ok(id, group)
				: error(id, -32004, 'device_not_found', 'Session group not found');
		}
		case 'sessionGroups.unlink':
			state.groups = state.groups.filter((item) => item.sessionGroupId !== params.sessionGroupId);
			return ok(id, { accepted: true });
		case 'alerts.listRules':
			return ok(id, state.rules);
		case 'alerts.replaceRules':
			state.rules = Array.isArray(params.rules) ? (params.rules as AlertRule[]) : [];
			return ok(id, state.rules);
		case 'logging.getStorageState':
			return ok(id, storageState());
		case 'vesc.getFirmwareInfo':
			if (!state.activeConnection) return error(id, -32000, 'invalid_state', 'No active board');
			return ok(id, {
				vescVersion: '6.06',
				packageName: 'refloat',
				packageVersion: '1.2',
				hardwareName: state.activeConnection.name,
				supported: true
			});
		default:
			return error(id, -32601, 'unsupported', `Unknown method: ${request.method}`);
	}
}

export function handleApiV1Payload(session: ApiV1Session, payload: string): ApiV1Response {
	try {
		return handleApiV1Request(session, JSON.parse(payload));
	} catch {
		return error(null, -32700, 'protocol_error', 'Invalid JSON');
	}
}
