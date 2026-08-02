export const MAX_MESSAGE_BYTES = 65_536;
export const MAX_SESSION_DATA_REQUEST_BYTES = 16_384;
export const MAX_SESSION_DATA_RESPONSE_BYTES = 4 * 1024 * 1024;
export const MAX_SESSION_DATA_FIELDS = 16;
export const MAX_SESSION_SERIES_BUCKETS = 1_024;
export const MAX_SESSION_SERIES_OBSERVATIONS = 65_536;
export const MAX_SESSION_SAMPLE_ROWS = 2_000;
export const MAX_SESSION_EVENT_ROWS = 500;
export const MAX_SESSION_CURSOR_BYTES = 512;
export const MAX_SESSION_LIST_PAGE = 100;
export const MAX_SESSION_LIST_CURSOR_BYTES = 1_024;
export const MAX_CAN_DISCOVERY_IDS = 64;
export const MAX_CONTROLLER_CANDIDATES = 16;
export const MAX_TELEMETRY_FIELDS = 64;
export const MAX_TELEMETRY_FIELD_BYTES = 64;
export const MAX_TELEMETRY_FIELD_NAME_BYTES = 4_096;
export const MAX_TELEMETRY_EXTENSIONS = 32;
export const MAX_TELEMETRY_JSON_BYTES = 16_384;

export type BluetoothRadioState = 'unknown' | 'unsupported' | 'powered_off' | 'powered_on' | 'resetting';

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

export type SessionFieldSource = 'stable' | 'extension';

export interface SessionFieldDescriptor {
  name: string;
  source: SessionFieldSource;
}

export interface SessionPublishedSnapshot {
  historyRevision: number;
  committedSampleSequence: number;
  committedEventSequence: number;
  committedElapsedMs: number;
}

export interface SessionPruneMetadata {
  targetResolutionSeconds: number | null;
  recordedSampleCount: number;
  storedSampleCount: number;
  policy: string | null;
}

export interface SessionMetadata {
  summary: SessionSummary;
  fields: SessionFieldDescriptor[];
  eventCount: number;
  recordedSampleCount: number;
  storedSampleCount: number;
  prune: SessionPruneMetadata;
  snapshot: SessionPublishedSnapshot;
}

export interface SessionListPage {
  items: SessionSummary[];
  catalogRevision: number;
  nextCursor: string | null;
}

export type SessionOperationKind = 'delete' | 'prunePreview' | 'prune';
export type SessionOperationStatus = 'queued' | 'running' | 'cancelling' | 'completed' | 'cancelled' | 'failed';

export interface SessionOperationState {
  operationId: string;
  kind: SessionOperationKind;
  status: SessionOperationStatus;
  sessionId: string | null;
  scope: 'one' | 'all';
  targetResolutionSeconds: number | null;
  completedSessions: number;
  eligibleSessions: number | null;
  error: BackendError | null;
}

export interface SessionPrunePreview {
  operation: SessionOperationState;
  recordedSampleCount: number;
  estimatedStoredSampleCount: number;
  estimatedReclaimableBytes: number | null;
  skipped: boolean;
  skipReason: string | null;
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
  minimumSessionDistance: number;
  themePreference: ThemePreference;
  unitPreference: UnitPreference;
  historyGapThresholdSeconds: number | null;
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
  sessionCatalogRevision: number;
  settings: Settings;
  sessionOperation?: SessionOperationState | null;
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
  'session.list': {
    params: { limit: number; cursor?: string | null };
    result: SessionListPage;
  };
  'session.get': { params: { sessionId: string }; result: SessionMetadata };
  'session.export': {
    params: { sessionId: string; format: 'float-view-archive' | 'csv' };
    result: ExportResult;
  };
  'session.delete': { params: { sessionId: string }; result: Accepted };
  'session.delete.cancel': { params: { operationId: string }; result: SessionOperationState };
  'session.prune.preview': {
    params: { scope: 'one' | 'all'; sessionId?: string; targetResolutionSeconds: number };
    result: SessionPrunePreview;
  };
  'session.prune.start': {
    params: { scope: 'one' | 'all'; sessionId?: string; targetResolutionSeconds: number };
    result: SessionOperationState;
  };
  'session.prune.cancel': { params: { operationId: string }; result: SessionOperationState };
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
