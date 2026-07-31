export const MAX_MESSAGE_BYTES = 65_536;
export const MAX_RECENT_ITEMS = 50;
export const MAX_SERIES_POINTS = 2_000;
export const MAX_SESSION_EVENTS = 2_000;
export const MAX_CAN_DISCOVERY_IDS = 64;
export const MAX_CONTROLLER_CANDIDATES = 16;
export const MAX_TELEMETRY_FIELDS = 64;
export const MAX_TELEMETRY_FIELD_BYTES = 64;
export const MAX_TELEMETRY_FIELD_NAME_BYTES = 4_096;
export const MAX_TELEMETRY_EXTENSIONS = 32;
export const MAX_TELEMETRY_JSON_BYTES = 16_384;

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
	| 'identifying'
	| 'selecting_controller'
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
	/** Test-version firmware is connected in read-only mode. */
	accessMode: 'readOnly' | 'readWrite';
	supported: boolean;
}

export type ControllerConnection = { kind: 'direct' } | { kind: 'can'; canId: number };

export interface ControllerCandidate {
	canId: number;
	hardwareName: string | null;
	firmwareName: string | null;
	vescVersion: string;
	vescSupported: boolean;
}

export type ConnectionIssueCode =
	| 'transport_setup_failed'
	| 'identification_timeout'
	| 'no_controller_found'
	| 'too_many_can_devices'
	| 'too_many_controllers'
	| 'preferred_controller_unavailable'
	| 'controller_changed'
	| 'unsupported_controller'
	| 'unsupported_refloat';

export interface ConnectionIssue {
	code: ConnectionIssueCode;
	message: string;
}

export type RideState = 'idle' | 'riding' | 'unknown';
export type PackageState = 'disabled' | 'startup' | 'ready' | 'running' | 'unknown';
export type PackageMode = 'normal' | 'handTest' | 'flywheel' | 'unknown';
export type FootpadState = 'none' | 'left' | 'right' | 'both' | 'unknown';
export type StopCondition =
	| 'none'
	| 'pitch'
	| 'roll'
	| 'switchHalf'
	| 'switchFull'
	| 'reverseStop'
	| 'quickStop'
	| 'unknown';
export type SetpointAdjustment =
	| 'none'
	| 'centering'
	| 'reverseStop'
	| 'speed'
	| 'duty'
	| 'error'
	| 'highVoltage'
	| 'lowVoltage'
	| 'temperature'
	| 'unknown';
export type AlertReason =
	| 'none'
	| 'lowVoltage'
	| 'highVoltage'
	| 'controllerTemperature'
	| 'motorTemperature'
	| 'current'
	| 'duty'
	| 'sensors'
	| 'lowBattery'
	| 'idle'
	| 'error'
	| 'speed'
	| 'cellUnderTemperature'
	| 'cellOverTemperature'
	| 'cellLowVoltage'
	| 'cellHighVoltage'
	| 'cellImbalance'
	| 'bmsConnection'
	| 'bmsOverTemperature'
	| 'firmwareFault'
	| 'unknown';
export type FirmwareFault =
	| 'overVoltage'
	| 'underVoltage'
	| 'driver'
	| 'absoluteOverCurrent'
	| 'controllerOverTemperature'
	| 'motorOverTemperature'
	| 'gateDriverOverVoltage'
	| 'gateDriverUnderVoltage'
	| 'mcuUnderVoltage'
	| 'watchdogReset'
	| 'encoderSpi'
	| 'encoderSinCosBelowMinimum'
	| 'encoderSinCosAboveMaximum'
	| 'flashCorruption'
	| 'currentSensorOffset1'
	| 'currentSensorOffset2'
	| 'currentSensorOffset3'
	| 'unbalancedCurrents'
	| 'brake'
	| 'resolverLossOfTracking'
	| 'resolverDegradationOfSignal'
	| 'resolverLossOfSignal'
	| 'appConfigFlashCorruption'
	| 'motorConfigFlashCorruption'
	| 'encoderNoMagnet'
	| 'encoderMagnetTooStrong'
	| 'phaseFilter'
	| 'unknown';

export interface LiveSample {
	sequence: number;
	observedAt: string;
	slowObservedAt: string | null;
	elapsedMs: number;
	rideState: RideState;
	packageState: PackageState | null;
	packageMode: PackageMode | null;
	footpadState: FootpadState | null;
	stopCondition: StopCondition | null;
	setpointAdjustment: SetpointAdjustment | null;
	wheelslip: boolean | null;
	darkride: boolean | null;
	charging: boolean | null;
	alertReason: AlertReason | null;
	firmwareFault: FirmwareFault | null;
	packVoltageV: number | null;
	dutyCycle: number | null;
	controllerTempC: number | null;
	motorTempC: number | null;
	motorCurrentA: number | null;
	batteryCurrentA: number | null;
	erpm: number | null;
	speedKph: number | null;
	pitchDeg: number | null;
	balancePitchDeg: number | null;
	rollDeg: number | null;
	directionalMotorCurrentA: number | null;
	filteredMotorCurrentA: number | null;
	requestedBalanceCurrentA: number | null;
	boosterCurrentA: number | null;
	fieldWeakeningCurrentA: number | null;
	footpadAdc1V: number | null;
	footpadAdc2V: number | null;
	remoteInput: number | null;
	setpointDeg: number | null;
	atrSetpointDeg: number | null;
	brakeTiltSetpointDeg: number | null;
	torqueTiltSetpointDeg: number | null;
	turnTiltSetpointDeg: number | null;
	remoteSetpointDeg: number | null;
	atrAccelerationDifferenceErpmPerSecond: number | null;
	atrSpeedBoost: number | null;
	distanceM: number | null;
	odometerM: number | null;
	ampHoursDrawnAh: number | null;
	ampHoursChargedAh: number | null;
	wattHoursDrawnWh: number | null;
	wattHoursChargedWh: number | null;
	batteryLevel: number | null;
	chargingCurrentA: number | null;
	chargingVoltageV: number | null;
	extensions: Record<string, number>;
}

export interface ActiveConnection extends Device {
	firmware: FirmwareInfo | null;
	savedDeviceId: string | null;
	displayName: string;
	controllerConnection: ControllerConnection | null;
	controllerCandidates: ControllerCandidate[];
	connectionIssue: ConnectionIssue | null;
	activeSession: SessionSummary | null;
	latestSample: LiveSample | null;
	currentLimits: ControllerCurrentLimits | null;
}

export interface ControllerCurrentLimits {
	motorCurrentMaxA: number;
	motorCurrentMinA: number;
	batteryCurrentMaxA: number;
	batteryCurrentMinA: number;
	batterySeriesCells: number | null;
}

export interface SavedDevice {
	savedDeviceId: string;
	deviceHistoryId: string;
	revision: number;
	displayName: string;
	customDisplayName?: string | null;
	advertisedName?: string | null;
	bluetoothIdentifier: string;
	lastSeenAt?: string | null;
	lastConnectedAt?: string | null;
	profileComplete: boolean;
}

export interface BoardProfileValues extends Record<string, unknown> {
	seriesCount?: number;
	parallelCount?: number;
	motor?: string;
	preferredCanId?: number | null;
}

export interface BoardProfile {
	savedDeviceId: string;
	revision: number;
	values: BoardProfileValues;
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

export type ThemePreference = 'system' | 'light' | 'dark';
export type UnitPreference = 'metric' | 'imperial';

export interface Settings {
	revision: number;
	idleProbeIntervalMs: number;
	idlePersistIntervalMs: number;
	ridingPersistIntervalMs: number;
	postRideTailMs: number;
	reconnectDurationMs: number;
	automaticSessionLinkingEnabled: boolean;
	sessionLinkWindowMs: number;
	themePreference: ThemePreference;
	unitPreference: UnitPreference;
}

export interface SeriesPoint {
	elapsedMs: number;
	values: Record<string, number | null>;
}

export type SessionEventKind =
	| 'packageState'
	| 'packageMode'
	| 'footpadState'
	| 'stopCondition'
	| 'setpointAdjustment'
	| 'wheelslip'
	| 'darkride'
	| 'charging'
	| 'alertReason'
	| 'firmwareFault';

export interface SessionEvent {
	elapsedMs: number;
	kind: SessionEventKind;
	value: string;
}

export interface SessionDetail {
	summary: SessionSummary;
	series: SeriesPoint[];
	events: SessionEvent[];
	eventsTruncated: boolean;
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
	'device.selectController': {
		params: { deviceId: string; canId: number };
		result: Accepted;
	};
	'device.disconnect': { params: EmptyParams; result: Accepted };
	'savedDevice.update': {
		params: {
			savedDeviceId: string;
			expectedRevision: number;
			customDisplayName: string | null;
		};
		result: SavedDevice;
	};
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
