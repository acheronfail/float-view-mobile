export const MAX_MESSAGE_BYTES = 65_536;
export const MAX_RECENT_ITEMS = 50;
export const MAX_SERIES_POINTS = 2_000;

export type BluetoothRadioState =
	| 'unknown'
	| 'unsupported'
	| 'powered_off'
	| 'powered_on'
	| 'resetting';

export type BluetoothPermissionState =
	| 'unknown'
	| 'not_required'
	| 'not_determined'
	| 'denied'
	| 'restricted'
	| 'granted';

export interface BluetoothState {
	radio: BluetoothRadioState;
	scanPermission: BluetoothPermissionState;
	connectPermission: BluetoothPermissionState;
	scanning: boolean;
}

export type DeviceState =
	| 'discovered'
	| 'connecting'
	| 'connected'
	| 'discovering_services'
	| 'ready'
	| 'disconnecting'
	| 'reconnecting'
	| 'disconnected'
	| 'failed';

export interface Device {
	/** Opaque backend-issued identifier. */
	deviceId: string;
	name: string;
	state: DeviceState;
	lastSeenAt?: string | null;
}

export interface FirmwareInfo {
	vescVersion: string;
	packageName: string;
	packageVersion: string;
	hardwareName?: string | null;
	supported: boolean;
}

export interface ActiveConnection extends Device {
	firmware: FirmwareInfo | null;
}

export interface SavedDevice {
	savedDeviceId: string;
	deviceHistoryId: string;
	displayName: string;
	advertisedName?: string | null;
	lastSeenAt?: string | null;
	lastConnectedAt?: string | null;
	profileComplete: boolean;
}

export interface BoardProfile {
	savedDeviceId: string;
	revision: number;
	values: Record<string, unknown>;
	missingFields: string[];
}

export type SessionState = 'active' | 'completed' | 'interrupted' | 'partial' | 'failed';

export interface SessionSummary {
	sessionId: string;
	deviceHistoryId: string;
	displayName: string;
	startedAt: string;
	endedAt?: string | null;
	state: SessionState;
	integrity: 'complete' | 'gaps' | 'recovered' | 'corrupt';
	sampleCount: number;
	eventCount: number;
	ridingElapsedMs: number;
	totalElapsedMs: number;
	distanceM?: number | null;
	maxSpeedMps?: number | null;
	maxDutyCycle?: number | null;
}

export interface SessionGroup {
	sessionGroupId: string;
	deviceHistoryId: string;
	sessionIds: string[];
	totalElapsedMs: number;
	distanceM?: number | null;
}

export interface Settings {
	revision: number;
	idleProbeIntervalMs: number;
	idlePersistIntervalMs: number;
	ridingPersistIntervalMs: number;
	postRideTailMs: number;
	reconnectDurationMs: number;
	automaticSessionLinkingEnabled: boolean;
	sessionLinkWindowMs: number;
	rawPacketCaptureEnabled: boolean;
}

export interface SeriesPoint {
	elapsedMs: number;
	values: Record<string, number | null>;
}

export interface SessionDetail {
	summary: SessionSummary;
	series: SeriesPoint[];
}

export interface ExportResult {
	fileName: string;
	delivery: 'presented' | 'staged';
}

export interface AppState {
	revision: number;
	bluetooth: BluetoothState;
	discoveredDevices: Device[];
	savedDevices: SavedDevice[];
	activeConnection: ActiveConnection | null;
	recentSessions: SessionSummary[];
	sessionGroups: SessionGroup[];
	settings: Settings;
}

export type BackendErrorCode =
	| 'invalid_request'
	| 'invalid_state'
	| 'permission_required'
	| 'not_found'
	| 'conflict'
	| 'unavailable'
	| 'internal';

export interface BackendError {
	code: BackendErrorCode;
	message: string;
	details?: Record<string, unknown>;
}

export type EmptyParams = Record<string, never>;
export interface Accepted {
	accepted: true;
}

export interface BackendMethods {
	'state.get': { params: EmptyParams; result: AppState };
	'bluetooth.requestPermission': { params: EmptyParams; result: BluetoothState };
	'bluetooth.scan': { params: { timeoutMs?: number }; result: Accepted };
	'device.connect': { params: { deviceId: string }; result: Accepted };
	'device.disconnect': { params: EmptyParams; result: Accepted };
	'settings.update': {
		params: { expectedRevision: number; patch: Omit<Partial<Settings>, 'revision'> };
		result: Settings;
	};
	'boardProfile.get': { params: { savedDeviceId: string }; result: BoardProfile };
	'boardProfile.update': {
		params: { savedDeviceId: string; expectedRevision: number; patch: Record<string, unknown> };
		result: BoardProfile;
	};
	'session.get': { params: { sessionId: string; maxPoints?: number }; result: SessionDetail };
	'session.getDiagnostics': {
		params: { sessionId: string; centerElapsedMs: number; beforeMs: number; afterMs: number };
		result: SeriesPoint[];
	};
	'session.export': {
		params: { sessionId: string; format: 'float-view-archive' | 'csv' };
		result: ExportResult;
	};
	'session.delete': { params: { sessionId: string }; result: Accepted };
	'sessionGroup.unlink': { params: { sessionGroupId: string }; result: Accepted };
}

export type BackendMethod = keyof BackendMethods;
export type BackendParams<M extends BackendMethod> = BackendMethods[M]['params'];
export type BackendResult<M extends BackendMethod> = BackendMethods[M]['result'];

export type Request<M extends BackendMethod = BackendMethod> = {
	id: number;
	method: M;
	params: BackendParams<M>;
};

export type Success = { id: number; result: unknown };
export type Failure = { id: number; error: BackendError };
export type Response = Success | Failure;
export type StateChanged = { event: 'state.changed'; data: AppState };
export type BackendEvent = StateChanged;
