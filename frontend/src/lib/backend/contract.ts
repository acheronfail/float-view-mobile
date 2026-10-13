export const MAX_MESSAGE_BYTES = 65_536;
export const MAX_SESSION_DATA_REQUEST_BYTES = 16_384;
export const MAX_SESSION_DATA_RESPONSE_BYTES = 4 * 1024 * 1024;
export const MAX_SESSION_IMPORT_BYTES = 128 * 1024 * 1024;
export const MAX_SESSION_DATA_FIELDS = 16;
export const MAX_SESSION_SERIES_BUCKETS = 1_024;
export const MAX_SESSION_SERIES_OBSERVATIONS = 65_536;
export const MAX_SESSION_SAMPLE_ROWS = 2_000;
export const MAX_SESSION_EVENT_ROWS = 500;
export const MAX_SESSION_LOCATION_ROWS = 2_000;
export const MAX_SESSION_LOCATION_GAPS = 2_000;
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
export const MAX_REFLOAT_CONFIGURATION_FIELDS = 512;
export const MAX_REFLOAT_CONFIGURATION_PAGE_FIELDS = 32;
export const MAX_REFLOAT_CONFIGURATION_GROUPS = 16;
export const MAX_REFLOAT_CONFIGURATION_SUBGROUPS = 64;
export const MAX_REFLOAT_CONFIGURATION_ENUM_CHOICES = 128;
export const MAX_REFLOAT_CONFIGURATION_NAME_BYTES = 128;
export const MAX_REFLOAT_CONFIGURATION_DESCRIPTION_BYTES = 4_096;
export const MAX_REFLOAT_CONFIGURATION_STRING_BYTES = 4_096;
export const MAX_REFLOAT_CONFIGURATION_CURSOR_BYTES = 512;
export const MAX_REFLOAT_CONFIGURATION_JSON_BYTES = 48 * 1_024;
export const MAX_REFLOAT_TUNE_NAME_BYTES = 128;
export const MAX_REFLOAT_TUNE_DESCRIPTION_BYTES = 1_024;
export const MAX_REFLOAT_TUNE_LIST_PAGE = 50;
export const MAX_REFLOAT_TUNE_CURSOR_BYTES = 512;

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

export interface DiscoveredDevice extends Device {
  /** Latest Bluetooth scan signal strength in dBm. */
  rssi: number | null;
  /** Saved board matched by the native platform association, when known. */
  savedDeviceId: string | null;
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

/** Identity of the Bluetooth peer, independent of a motor reached through CAN.
 * Public firmware metadata only; no Bluetooth address or controller UUID. */
export interface TransportFirmwareInfo {
  vescVersion: string;
  hardwareName: string | null;
  firmwareName: string | null;
  testVersion: number;
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
  /** Session timeline, including idle and disconnected time. */
  elapsedMs: number;
  /** Native-owned accumulated time between consecutive riding observations; offline gaps excluded. */
  ridingElapsedMs: number;
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
  /** Raw controller-reported fraction, preserved in recordings and exports. */
  batteryLevel: number | null;
  chargingCurrentA: number | null;
  chargingVoltageV: number | null;
  extensions: Record<string, number>;
}

export interface BoardHistorySummary {
  sessionCount: number;
  totalRidingElapsedMs: number;
  totalDistanceM: number | null;
  mostRecentSessionAt: string | null;
}

export interface LiveLocation {
  latitude: number;
  longitude: number;
  accuracyM: number | null;
  observedAt: string;
}

export type BmsStatus = 'discovering' | 'not_found' | 'attached' | 'unavailable' | 'ambiguous';

export type BmsConnection = { kind: 'direct' } | { kind: 'can'; canId: number };

export interface BmsCandidate {
  connection: BmsConnection;
  hardwareName: string | null;
  vescVersion: string;
}

/** Complete live-only VBMS values. Fractions use 0...1; humidity uses percent. */
export interface BmsSnapshot {
  observedAt: string;
  totalVoltageV: number;
  chargeVoltageV: number;
  inputCurrentA: number;
  inputIcCurrentA: number;
  sessionAmpHoursAh: number;
  sessionWattHoursWh: number;
  cellVoltagesV: number[];
  balancing: boolean[];
  temperaturesC: number[];
  icTemperatureC: number;
  humiditySensorTemperatureC: number;
  humidityPercent: number;
  highestCellTemperatureC: number;
  /** Highest reported BMS circuitry temperature, excluding cell probes. */
  highestCircuitryTemperatureC: number;
  stateOfCharge: number;
  stateOfHealth: number;
  reportedCanId: number;
  lifetimeChargeAmpHoursAh: number;
  lifetimeChargeWattHoursWh: number;
  lifetimeDischargeAmpHoursAh: number;
  lifetimeDischargeWattHoursWh: number;
  pressurePa: number;
  dataVersion: number | null;
  status: string | null;
}

export interface BmsState {
  status: BmsStatus;
  connection: BmsConnection | null;
  candidates: BmsCandidate[];
  latestSnapshot: BmsSnapshot | null;
}

export type RefloatControl = 'package' | 'flywheel' | 'leds' | 'headlights';
export type RefloatPendingAction = RefloatControl | 'clearFatalError';

/** Complete authoritative, connection-scoped Refloat control state. */
export interface RefloatControls {
  status: 'loading' | 'ready' | 'unavailable';
  writable: boolean;
  packageEnabled: boolean | null;
  flywheelEnabled: boolean | null;
  lightingAvailable: boolean;
  ledsEnabled: boolean | null;
  headlightsEnabled: boolean | null;
  fatalErrorClearAvailable: boolean;
  fatalErrorLatched: boolean | null;
  fatalErrorActive: boolean | null;
  fatalErrorClearable: boolean | null;
  pendingAction: RefloatPendingAction | null;
}

/** Complete authoritative, connection-scoped VESC controller actions. */
export interface ControllerActions {
  shutdownAvailable: boolean;
  pendingAction: 'shutdown' | null;
}

export type RefloatConfigurationAvailability = 'loading' | 'ready' | 'unavailable';
export type RefloatConfigurationOperation = 'schema' | 'readCurrent' | 'readDefault' | 'write' | 'applyTune';

export interface RefloatConfigurationSummary {
  availability: RefloatConfigurationAvailability;
  writable: boolean;
  packageVersion: string | null;
  schemaFingerprint: string | null;
  wireSignature: number | null;
  currentLoaded: boolean;
  defaultLoaded: boolean;
  pendingOperation: RefloatConfigurationOperation | null;
}

export interface RefloatConfigurationIdentity {
  packageName: 'refloat';
  packageVersion: string;
  schemaFingerprint: string;
  wireSignature: number;
}

export type RefloatConfigurationValue =
  | { type: 'double'; value: number }
  | { type: 'integer'; value: number }
  | { type: 'boolean'; value: boolean }
  | { type: 'string'; value: string };

export type RefloatConfigurationFieldType = 'double' | 'integer' | 'enum' | 'boolean' | 'string' | 'bitfield';

export interface RefloatConfigurationField {
  name: string;
  label: string;
  type: RefloatConfigurationFieldType;
  defaultValue: RefloatConfigurationValue;
  minimum: number | null;
  maximum: number | null;
  step: number | null;
  precision: number | null;
  suffix: string;
  choices: string[];
  description: string;
}

export interface RefloatConfigurationSchemaPage {
  identity: RefloatConfigurationIdentity;
  group: string;
  subgroup: string;
  fields: RefloatConfigurationField[];
  nextCursor: string | null;
}

export interface RefloatConfigurationValues {
  identity: RefloatConfigurationIdentity;
  revision: number;
  source: 'current' | 'default';
  values: Record<string, RefloatConfigurationValue>;
}

/** Durable editor work; saving this never changes device configuration. */
export interface RefloatEditorDraft {
  version: 1;
  identity: RefloatConfigurationIdentity;
  source: 'current' | 'default';
  values: Record<string, RefloatConfigurationValue>;
  inputs: Record<string, string>;
  position: {
    sectionKey: string;
    search: string;
    controlsExpanded: boolean;
    scrollTop: number;
  };
}
export const MAX_REFLOAT_EDITOR_DRAFT_BYTES = 60_000;

export interface RefloatTuneSummary {
  tuneId: string;
  revision: number;
  name: string;
  description: string | null;
  identity: RefloatConfigurationIdentity;
  createdAt: string;
  updatedAt: string;
}

export interface RefloatTune extends RefloatTuneSummary {
  values: Record<string, RefloatConfigurationValue>;
}

export interface RefloatTuneListPage {
  items: RefloatTuneSummary[];
  nextCursor: string | null;
}

/** Shared temporal charge approximation. No view-local estimation or filtering.
 * level is a nullable fraction [0,1]; seriesCount resolves pack volts to cell volts.
 * sampleSequence/profileRevision identify the accepted sample and saved configuration.
 * catalogVersion pins community curves; modes describe compensation, not accuracy.
 */
export interface BatteryEstimate {
  level: number | null;
  /** Null when unset or not applicable (guest/BMS). */
  profileId: string | null;
  /** Saved series count, null when absent or conflicting with the controller. */
  seriesCount: number | null;
  /** Null before the first accepted sample. */
  sampleSequence: number | null;
  /** Null without a saved profile. */
  profileRevision: number | null;
  catalogVersion: string;
  /** Describes this update, independently of voltage compensation mode. */
  method: 'voltage_smoothed' | 'charge_integrated' | null;
  provisional: boolean;
  mode: 'load_compensated' | 'load_clamped' | 'voltage_only' | 'unavailable';
  unavailableReason: 'not_configured' | 'series_mismatch' | 'missing_voltage' | 'not_applicable' | null;
}
/** Service-owned named pack variant. Selecting its ID atomically applies defaults. */
export interface BatteryPreset {
  id: string;
  label: string;
  manufacturer: string;
  cellLabel: string;
  seriesCount: number;
  parallelCount: number;
  batteryCellProfileId: string;
  nominalCapacityAh: number | null;
}
export interface BatteryCellProfile {
  id: string;
  label: string;
}
export interface ActiveConnection extends Device {
  deviceKind: 'board' | 'bms' | null;
  firmware: FirmwareInfo | null;
  transportFirmware: TransportFirmwareInfo | null;
  trackingMode: 'saved' | 'guest';
  savedDeviceId: string | null;
  displayName: string;
  controllerConnection: ControllerConnection | null;
  controllerCandidates: ControllerCandidate[];
  connectionIssue: ConnectionIssue | null;
  bms: BmsState;
  /** Null only before Refloat discovery begins. Never persisted. */
  refloatControls: RefloatControls | null;
  /** Small authoritative status only; schema, values, and Tunes remain request-scoped. */
  refloatConfiguration: RefloatConfigurationSummary | null;
  /** Live-only VESC controller actions. Never persisted. */
  controllerActions: ControllerActions;
  activeSession: SessionSummary | null;
  latestSample: LiveSample | null;
  batteryEstimate: BatteryEstimate;
  /** Saved board layout, or the separate guest layout. */
  liveWidgetSlots: LiveWidgetSlots;
  latestLocation: LiveLocation | null;
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
  bluetoothIdentifier: string | null;
  lastSeenAt?: string | null;
  lastConnectedAt?: string | null;
  profileComplete: boolean;
  summary: BoardHistorySummary;
}

export interface BoardProfileValues extends Record<string, unknown> {
  seriesCount?: number;
  parallelCount?: number;
  batteryCellProfileId?: string | null;
  /** Full pack Ah, 0.1–200; null means voltage-only estimation. */
  batteryCapacityAh?: number | null;
  /** Non-null in a patch selects all defaults; cannot accompany battery overrides.
   * Complete imported profiles preserve overrides and must match S/P/curve. */
  batteryPresetId?: string | null;
  /** Display-empty volts per cell, [2.7,4.2); absent/null means 3.0 V. */
  batteryMinimumCellVoltageV?: number | null;
  motor?: string;
  preferredCanId?: number | null;
  preferredBmsCanId?: number | null;
  /** Absent means the default layout. Always exactly five entries when set. */
  liveWidgetSlots?: LiveWidgetSlots;
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
  committedLocationSequence: number;
  committedLocationElapsedMs: number;
}

export type LocationAuthorizationStatus =
  | 'disabled'
  | 'notDetermined'
  | 'foregroundOnly'
  | 'authorized'
  | 'denied'
  | 'restricted'
  | 'unavailable';

export type LocationPermissionAction = 'requestPermission' | 'openSettings' | null;

/** Complete native inventory, at most three unique rows. Labels are frontend-owned. */
export type PermissionId = 'bluetooth' | 'location' | 'backgroundLocation' | 'notifications' | 'liveUpdates';
export type PermissionAction = 'requestPermission' | 'openSettings';
export type PermissionGrant = 'unknown' | 'notDetermined' | 'granted' | 'denied' | 'restricted' | 'notRequired';
export type PermissionGuidance =
  | 'none'
  | 'checking'
  | 'restricted'
  | 'servicesDisabled'
  | 'unsupportedHardware'
  | 'foregroundRequired'
  | 'rationale'
  | 'appSettings'
  | 'iosSettings'
  | 'macBluetooth'
  | 'macLocation'
  | 'backgroundSettings'
  | 'alwaysAccess'
  | 'fineLocationForScan'
  | 'notificationSettings'
  | 'liveUpdateSettings'
  | 'notificationsRequired';
export interface PermissionRow {
  permission: PermissionId;
  status: 'notDetermined' | 'granted' | 'limited' | 'denied' | 'restricted' | 'unavailable';
  action: PermissionAction | null;
  guidance: PermissionGuidance;
  pending: boolean;
  optional: boolean;
  servicesEnabled: boolean | null;
  scan: PermissionGrant | null;
  connect: PermissionGrant | null;
  accuracy: 'unknown' | 'approximate' | 'precise';
  scope: 'unknown' | 'whileInUse' | 'always';
  /** OS-localized background setting label, at most 120 characters. */
  optionLabel: string | null;
}

export interface LocationState {
  status: LocationAuthorizationStatus;
  permissionAction: LocationPermissionAction;
  backgroundCapture: 'available' | 'foregroundOnly' | 'unavailable';
  reason: string | null;
}

export interface SessionLocationMetadata {
  available: boolean;
  count: number;
  lastSequence: number;
  lastElapsedMs: number;
  status: 'available' | 'disabled' | 'unavailable' | 'gap';
}

export interface SessionLocationPoint {
  locationSequence: number;
  elapsedMs: number;
  latitude: number;
  longitude: number;
  accuracyM: number | null;
  observedAt: string;
}

export interface SessionLocationGap {
  startElapsedMs: number;
  endElapsedMs: number;
  reason: 'providerUnavailable' | 'permissionUnavailable' | 'noFix' | 'unknown';
}

export interface SessionPruneMetadata {
  targetResolutionSeconds: number | null;
  recordedSampleCount: number;
  storedSampleCount: number;
  policy: string | null;
}

export const MAX_SESSION_MARKERS = 1_000;
export const MAX_SESSION_MARKER_ROWS = 256;
export interface SessionRideMarker {
  sequence: number;
  elapsedMs: number;
  observedAtMs: number;
  operationId: string;
  location: {
    locationSequence: number;
    elapsedMs: number;
    observedAtMs: number;
    latitude: number;
    longitude: number;
    accuracyM: number | null;
  } | null;
}

export interface SessionMetadata {
  markerCount: number;
  summary: SessionSummary;
  fields: SessionFieldDescriptor[];
  eventCount: number;
  recordedSampleCount: number;
  storedSampleCount: number;
  prune: SessionPruneMetadata;
  location: SessionLocationMetadata;
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

export const liveWidgetFields = [
  'empty',
  'headlights',
  'leds',
  'fieldWeakeningCurrentA',
  'motorCurrentA',
  'batteryCurrentA',
  'powerW',
  'packVoltageV',
  'dutyCycle',
  'driveLoad',
  'estimatedBatteryLevel',
  'batteryLevel',
  'speedKph',
  'distanceM',
  'odometerM',
  'elapsedMs',
  'erpm',
  'controllerTempC',
  'motorTempC',
  'highestTempC',
  'bmsStateOfCharge',
  'bmsStateOfHealth',
  'bmsTotalVoltageV',
  'bmsInputCurrentA',
  'bmsChargeVoltageV',
  'bmsMinCellVoltageV',
  'bmsMaxCellVoltageV',
  'bmsCellVoltageSpreadV',
  'bmsHighestCellTempC',
  'bmsIcTempC',
  'bmsHumidityPercent',
  'directionalMotorCurrentA',
  'filteredMotorCurrentA',
  'requestedBalanceCurrentA',
  'boosterCurrentA',
  'pitchDeg',
  'balancePitchDeg',
  'rollDeg',
  'footpadAdc1V',
  'footpadAdc2V',
  'remoteInput',
  'setpointDeg',
  'atrSetpointDeg',
  'brakeTiltSetpointDeg',
  'torqueTiltSetpointDeg',
  'turnTiltSetpointDeg',
  'remoteSetpointDeg',
  'atrAccelerationDifferenceErpmPerSecond',
  'atrSpeedBoost',
  'ampHoursDrawnAh',
  'ampHoursChargedAh',
  'wattHoursDrawnWh',
  'wattHoursChargedWh',
  'chargingCurrentA',
  'chargingVoltageV',
] as const;
export type LiveWidgetField = (typeof liveWidgetFields)[number];
export type LiveWidgetSlots = [LiveWidgetField, LiveWidgetField, LiveWidgetField, LiveWidgetField, LiveWidgetField];
export const defaultLiveWidgetSlots: LiveWidgetSlots = ['empty', 'powerW', 'elapsedMs', 'odometerM', 'distanceM'];

export interface SessionDisplayOptions {
  bothFootpadsOff: boolean;
  oneFootpadOff: boolean;
}

export interface Settings {
  sessionDisplayOptions: SessionDisplayOptions;
  revision: number;
  idleProbeIntervalMs: number;
  idlePersistIntervalMs: number;
  ridingPersistIntervalMs: number;
  postRideTailMs: number;
  reconnectDurationMs: number;
  minimumSessionDistance: number;
  themePreference: ThemePreference;
  unitPreference: UnitPreference;
  /** Layout used only for guest connections; saved boards use BoardProfileValues. */
  liveWidgetSlots: LiveWidgetSlots;
  historyGapThresholdSeconds: number | null;
  locationRecordingEnabled: boolean;
  includeGpsLocationWhenExporting: boolean;
}

export interface ExportResult {
  fileName: string;
  delivery: 'presented' | 'staged';
}

export interface AppState {
  /** Seven shared-service choices; numeric curves never cross the contract. */
  batteryCellProfiles: BatteryCellProfile[];
  batteryPresets: BatteryPreset[];
  batteryPresetCatalogVersion: string;
  revision: number;
  permissions: PermissionRow[];
  bluetooth: BluetoothState;
  discoveredDevices: DiscoveredDevice[];
  savedDevices: SavedDevice[];
  activeConnection: ActiveConnection | null;
  sessionCatalogRevision: number;
  settings: Settings;
  location: LocationState;
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

/** Process-memory presentation only; never product or device authority. */
export type UiRoute =
  | { kind: 'home' | 'sessions' | 'settings' | 'live' | 'bms' }
  | { kind: 'boardHistory'; savedDeviceId: string }
  | { kind: 'session'; sessionId: string; savedDeviceId: string | null };
export type UiSurface =
  | { kind: 'liveActions' | 'liveCharts' | 'bmsDetails' | 'refloatConfiguration' | 'refloatTunes' | 'boardActions' }
  | { kind: 'boardConfiguration'; savedDeviceId: string; batteryOnly: boolean };
export interface UiCheckpoint {
  version: 1;
  route: UiRoute;
  live: { deviceId: string; mode: 'normal' | 'detailed' } | null;
  surface: UiSurface | null;
}
export const MAX_UI_CHECKPOINT_BYTES = 4096;
export const MAX_UI_TARGET_BYTES = 512;

export interface BackendMethods {
  'state.get': { params: EmptyParams; result: AppState };
  'uiState.get': { params: EmptyParams; result: UiCheckpoint | null };
  'uiState.set': { params: { checkpoint: UiCheckpoint | null }; result: Accepted };
  /** Bounded, best-effort UI timing summary; never changes public state. */
  'diagnostics.report': {
    params: { snapshots: number; max_sample_age_ms: number; max_delivery_gap_ms: number; max_handler_ms: number };
    result: { accepted: true };
  };
  'permissions.recover': { params: { permission: PermissionId; action: PermissionAction }; result: PermissionRow[] };
  'bluetooth.requestPermission': { params: EmptyParams; result: BluetoothState };
  'bluetooth.scan': { params: { timeoutMs?: number }; result: Accepted };
  'location.requestPermission': { params: EmptyParams; result: LocationState };
  'location.openSettings': { params: EmptyParams; result: LocationState };
  'device.connect': {
    params: { deviceId: string; trackingMode: 'saved' | 'guest' };
    result: Accepted;
  };
  'device.selectController': {
    params: { deviceId: string; canId: number };
    result: Accepted;
  };
  'device.disconnect': { params: EmptyParams; result: Accepted };
  'controller.shutdown': { params: EmptyParams; result: Accepted };
  'refloat.control.set': {
    params: { control: RefloatControl; enabled: boolean };
    result: RefloatControls;
  };
  'refloat.fatalError.clear': { params: EmptyParams; result: RefloatControls };
  'refloat.configuration.draft.get': {
    params: { deviceId: string };
    result: RefloatEditorDraft | null;
  };
  'refloat.configuration.draft.set': {
    params: { deviceId: string; baseRevision: number; draft: RefloatEditorDraft };
    result: Accepted;
  };
  'refloat.configuration.open': { params: EmptyParams; result: RefloatConfigurationSummary };
  'refloat.configuration.schema': {
    params: { cursor?: string | null };
    result: RefloatConfigurationSchemaPage;
  };
  'refloat.configuration.read': {
    /** Defaults to a board read. False reuses valid retained values, reading only when absent. */
    params: { source: 'current' | 'default'; refresh?: boolean };
    result: RefloatConfigurationValues;
  };
  'refloat.configuration.write': {
    params: {
      packageVersion: string;
      schemaFingerprint: string;
      wireSignature: number;
      baseRevision: number;
      values: Record<string, RefloatConfigurationValue>;
    };
    result: RefloatConfigurationValues;
  };
  'refloat.tune.list': {
    params: { identity: RefloatConfigurationIdentity; limit: number; cursor?: string | null };
    result: RefloatTuneListPage;
  };
  'refloat.tune.get': { params: { tuneId: string }; result: RefloatTune };
  'refloat.tune.create': {
    params: {
      name: string;
      description: string | null;
      identity: RefloatConfigurationIdentity;
      sourceRevision: number;
      values: Record<string, RefloatConfigurationValue>;
    };
    result: RefloatTune;
  };
  'refloat.tune.update': {
    params: {
      tuneId: string;
      expectedRevision: number;
      name: string;
      description: string | null;
      values: Record<string, RefloatConfigurationValue>;
    };
    result: RefloatTune;
  };
  'refloat.tune.delete': { params: { tuneId: string; expectedRevision: number }; result: Accepted };
  'refloat.tune.apply': {
    params: { tuneId: string; expectedRevision: number; baseRevision: number };
    result: RefloatConfigurationValues;
  };
  'savedDevice.update': {
    params: {
      savedDeviceId: string;
      expectedRevision: number;
      customDisplayName: string | null;
    };
    result: SavedDevice;
  };
  'savedDevice.delete': { params: { savedDeviceId: string }; result: Accepted };
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
    params: { savedDeviceId: string; limit: number; cursor?: string | null };
    result: SessionListPage;
  };
  'session.marker.add': {
    params: { sessionId: string; operationId: string };
    result: { marker: SessionRideMarker; snapshot: SessionPublishedSnapshot };
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
