import { validUiCheckpoint } from '../src/lib/backend/ui-checkpoint.ts';
import type { UiCheckpoint } from '../src/lib/backend/contract.ts';
import { permissionInventory, locationSummary } from './permissions.ts';
import { batteryPresets, batteryPresetCatalogVersion, applyBatteryPatch } from './battery-presets.ts';
import { batteryCellProfiles, emptyEstimate, MockBattery } from './battery.ts';
import {
  defaultLiveWidgetSlots,
  liveWidgetFields,
  MAX_MESSAGE_BYTES,
  MAX_REFLOAT_CONFIGURATION_CURSOR_BYTES,
  MAX_REFLOAT_CONFIGURATION_JSON_BYTES,
  MAX_REFLOAT_TUNE_DESCRIPTION_BYTES,
  MAX_REFLOAT_TUNE_CURSOR_BYTES,
  MAX_REFLOAT_TUNE_LIST_PAGE,
  MAX_REFLOAT_TUNE_NAME_BYTES,
  MAX_SESSION_LIST_CURSOR_BYTES,
  MAX_SESSION_LIST_PAGE,
  type AppState,
  type ActiveConnection,
  type LiveWidgetSlots,
  type BoardHistorySummary,
  type BackendError,
  type BackendMethod,
  type BmsCandidate,
  type BmsSnapshot,
  type BmsState,
  type ConnectionIssueCode,
  type ControllerCandidate,
  type Request,
  type RefloatControl,
  type RefloatControls,
  type RefloatConfigurationField,
  type RefloatConfigurationIdentity,
  type RefloatConfigurationValue,
  type RefloatTune,
  type RefloatPendingAction,
  type Response,
  type SessionMetadata,
  type SessionRideMarker,
  type SessionListPage,
  type SessionOperationState,
  type SessionPrunePreview,
  type SessionSummary,
  type StateChanged,
} from '../src/lib/backend/contract.ts';

const refloatIdentity: RefloatConfigurationIdentity = {
  packageName: 'refloat',
  packageVersion: '1.3.0',
  schemaFingerprint: '62681208dc609eb19420c89df65105f018499aa9f5741f295f00d62ea5a1bceb',
  wireSignature: 421_162_011,
};

const refloatFields: RefloatConfigurationField[] = [
  {
    name: 'tiltback_duty',
    label: 'Duty Tiltback',
    type: 'double',
    defaultValue: { type: 'double', value: 0.8 },
    minimum: 0.5,
    maximum: 0.95,
    step: 0.01,
    precision: 2,
    suffix: '',
    choices: [],
    description: 'Duty-cycle threshold.',
  },
  {
    name: 'leds.on',
    label: 'LEDs On',
    type: 'boolean',
    defaultValue: { type: 'boolean', value: true },
    minimum: null,
    maximum: null,
    step: null,
    precision: null,
    suffix: '',
    choices: [],
    description: 'Persistent startup default.',
  },
  {
    name: 'leds.headlights_on',
    label: 'Headlights On',
    type: 'boolean',
    defaultValue: { type: 'boolean', value: true },
    minimum: null,
    maximum: null,
    step: null,
    precision: null,
    suffix: '',
    choices: [],
    description: 'Persistent startup default.',
  },
  {
    name: 'meta.is_default',
    label: 'Default Marker',
    type: 'boolean',
    defaultValue: { type: 'boolean', value: true },
    minimum: null,
    maximum: null,
    step: null,
    precision: null,
    suffix: '',
    choices: [],
    description: '',
  },
];

const defaultRefloatValues = Object.fromEntries(
  refloatFields.map((field) => [field.name, field.defaultValue])
) as Record<string, RefloatConfigurationValue>;

const readyRefloatControls = (overrides: Partial<RefloatControls> = {}): RefloatControls => ({
  status: 'ready',
  writable: true,
  packageEnabled: true,
  flywheelEnabled: false,
  lightingAvailable: true,
  ledsEnabled: true,
  headlightsEnabled: true,
  fatalErrorClearAvailable: true,
  fatalErrorLatched: true,
  fatalErrorActive: false,
  fatalErrorClearable: true,
  pendingAction: null,
  ...overrides,
});

const discoveredBoard = {
  deviceId: 'mock-board',
  name: 'Float View Test VESC',
  state: 'discovered' as const,
  rssi: -48,
  lastSeenAt: '2026-01-01T00:00:00Z',
  savedDeviceId: 'saved-board' as string | null,
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
  profileComplete: false,
  summary: {
    sessionCount: 2,
    totalRidingElapsedMs: 100_000,
    totalDistanceM: 600,
    mostRecentSessionAt: '2026-01-01T00:02:00Z',
  } satisfies BoardHistorySummary,
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
  eventCount: 2,
  ridingElapsedMs: 50_000,
  totalElapsedMs: 60_000,
  distanceM: 300,
  maxSpeedMps: 8,
  maxDutyCycle: 0.8,
};

const secondSession: SessionSummary = {
  ...session,
  sessionId: 'session-2',
  startedAt: '2026-01-01T00:02:00Z',
  endedAt: '2026-01-01T00:03:00Z',
};

const sessionFields = Object.keys(detailedSeriesValues({})).map((name) => ({
  name,
  source: 'stable' as const,
}));

function sessionMetadata(summary: SessionSummary): SessionMetadata {
  const hasLocation = summary.sessionId === 'session-1';
  return {
    summary,
    markerCount: 0,
    fields: sessionFields,
    eventCount: summary.eventCount,
    recordedSampleCount: summary.sampleCount,
    storedSampleCount: summary.sampleCount,
    prune: {
      targetResolutionSeconds: null,
      recordedSampleCount: summary.sampleCount,
      storedSampleCount: summary.sampleCount,
      policy: null,
    },
    snapshot: {
      historyRevision: 0,
      committedSampleSequence: Math.max(0, summary.sampleCount - 1),
      committedEventSequence: Math.max(0, summary.eventCount - 1),
      committedElapsedMs: summary.totalElapsedMs,
      committedLocationSequence: hasLocation ? 3 : 0,
      committedLocationElapsedMs: hasLocation ? 50_000 : 0,
    },
    location: hasLocation
      ? { available: true, count: 3, lastSequence: 3, lastElapsedMs: 50_000, status: 'available' }
      : { available: false, count: 0, lastSequence: 0, lastElapsedMs: 0, status: 'disabled' },
  };
}

function detailedSeriesValues(overrides: Record<string, number>): Record<string, number | null> {
  return {
    speedKph: 18,
    erpm: 2_900,
    pitchDeg: 1.4,
    balancePitchDeg: 1.1,
    rollDeg: 0.6,
    packVoltageV: 58.4,
    dutyCycle: 0.42,
    motorCurrentA: 18.6,
    directionalMotorCurrentA: 17.9,
    filteredMotorCurrentA: 17.4,
    batteryCurrentA: 8.4,
    requestedBalanceCurrentA: 20.2,
    boosterCurrentA: 1.2,
    fieldWeakeningCurrentA: 0,
    controllerTempC: 38.2,
    motorTempC: 41.7,
    footpadAdc1V: 2.8,
    footpadAdc2V: 2.75,
    remoteInput: 0,
    setpointDeg: 1,
    atrSetpointDeg: 0.4,
    brakeTiltSetpointDeg: 0,
    torqueTiltSetpointDeg: 0.6,
    turnTiltSetpointDeg: 0,
    remoteSetpointDeg: 0,
    atrAccelerationDifferenceErpmPerSecond: 180,
    atrSpeedBoost: 0.05,
    distanceM: 0,
    odometerM: 12_345,
    ampHoursDrawnAh: 18.4,
    ampHoursChargedAh: 2.7,
    wattHoursDrawnWh: 812,
    wattHoursChargedWh: 119,
    batteryLevel: 0.68,
    chargingCurrentA: 0,
    chargingVoltageV: 0,
    ...overrides,
  };
}

export class MockBackend {
  private uiCheckpoint: UiCheckpoint | null = null;
  private rideMarkers = new Map<string, SessionRideMarker[]>();
  markerRows(sessionId: string) {
    return structuredClone(this.rideMarkers.get(sessionId) ?? []);
  }

  private readonly publish: (event: StateChanged) => void;
  private readonly battery = new MockBattery();
  readonly profiles = new Map([
    [
      savedBoard.savedDeviceId,
      {
        savedDeviceId: savedBoard.savedDeviceId,
        revision: 0,
        values: { preferredBmsCanId: null } as Record<string, unknown>,
        missingFields: ['seriesCount', 'parallelCount', 'motor'],
      },
    ],
  ]);
  private nextSession = 3;
  private sessionCatalog: SessionSummary[] = [secondSession, session];
  private activeSessionId?: string;
  private pendingCandidates: ControllerCandidate[] = [];
  private discoveryExpiryTimer?: ReturnType<typeof setTimeout>;
  private refloatTimeoutOnce = false;
  private shutdownOutcome: 'no-op' | 'disconnect' = 'no-op';
  private refloatCurrentRevision = 1;
  private refloatCurrentValues = {
    ...defaultRefloatValues,
    'meta.is_default': { type: 'boolean', value: false },
  } satisfies Record<string, RefloatConfigurationValue>;
  private nextTune = 1;
  private readonly tunes = new Map<string, RefloatTune>();
  private state: AppState = {
    revision: 0,
    bluetooth: {
      radio: 'powered_on',
      scanPermission: 'granted',
      connectPermission: 'granted',
      scanning: false,
    },
    discoveredDevices: [],
    savedDevices: [savedBoard],
    activeConnection: null,
    sessionCatalogRevision: 0,
    batteryCellProfiles,
    batteryPresets,
    batteryPresetCatalogVersion,
    settings: {
      revision: 0,
      idleProbeIntervalMs: 500,
      idlePersistIntervalMs: 1_000,
      ridingPersistIntervalMs: 100,
      postRideTailMs: 5_000,
      reconnectDurationMs: 300_000,
      minimumSessionDistance: 100,
      themePreference: 'system',
      unitPreference: 'metric',
      sessionDisplayOptions: { bothFootpadsOff: false, oneFootpadOff: false },
      liveWidgetSlots: [...defaultLiveWidgetSlots],
      historyGapThresholdSeconds: null,
      locationRecordingEnabled: false,
      includeGpsLocationWhenExporting: false,
    },
    permissions: permissionInventory('ios'),
    location: {
      status: 'disabled',
      permissionAction: 'requestPermission',
      backgroundCapture: 'unavailable',
      reason: null,
    },
    sessionOperation: null,
  };

  private operation?: SessionOperationState;
  private operationTimer?: ReturnType<typeof setTimeout>;

  constructor(publish: (event: StateChanged) => void = () => {}) {
    this.publish = publish;
  }

  handlePayload(payload: string): Response {
    if (Buffer.byteLength(payload, 'utf8') > MAX_MESSAGE_BYTES) {
      return failure(0, 'invalid_request', 'Message is too large');
    }
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
      case 'diagnostics.report': {
        const limits = {
          snapshots: 10_000,
          max_sample_age_ms: 86_400_000,
          max_delivery_gap_ms: 86_400_000,
          max_handler_ms: 60_000,
        };
        if (
          Object.keys(params).length !== 4 ||
          Object.entries(limits).some(
            ([key, limit]) =>
              !Number.isSafeInteger(params[key]) || (params[key] as number) < 0 || (params[key] as number) > limit
          )
        )
          return failure(id, 'invalid_request', 'Invalid diagnostics summary');
        return success(id, { accepted: true });
      }
      case 'uiState.get':
        if (Object.keys(params).length) return failure(id, 'invalid_request', 'Invalid checkpoint request');
        return success(id, structuredClone(this.uiCheckpoint));
      case 'uiState.set':
        if (
          Object.keys(params).length !== 1 ||
          !('checkpoint' in params) ||
          (params.checkpoint !== null && !validUiCheckpoint(params.checkpoint))
        )
          return failure(id, 'invalid_request', 'Invalid checkpoint');
        this.uiCheckpoint = structuredClone(params.checkpoint as UiCheckpoint | null);
        return success(id, { accepted: true });
      case 'state.get':
        return success(id, this.state);
      case 'permissions.recover': {
        if (
          Object.keys(params).some((key) => !['permission', 'action'].includes(key)) ||
          !['bluetooth', 'location', 'backgroundLocation', 'notifications', 'liveUpdates'].includes(
            String(params.permission)
          ) ||
          !['requestPermission', 'openSettings'].includes(String(params.action))
        )
          return failure(id, 'invalid_request', 'Unknown permission or action');
        if (this.state.permissions.some((row) => row.pending))
          return failure(id, 'conflict', 'Permission presentation is pending');
        const row = this.state.permissions.find((row) => row.permission === params.permission);
        if (!row || row.action !== params.action)
          return failure(id, 'invalid_state', 'Permission action changed. Refresh and try again.');
        // Presentation never invents an OS grant. Tests explicitly supply later observations.
        return success(id, this.state.permissions);
      }
      case 'bluetooth.requestPermission':
        return success(id, this.state.bluetooth);
      case 'location.requestPermission':
        if (this.state.permissions.some((row) => row.pending))
          return failure(id, 'conflict', 'Permission presentation is pending');
        return success(id, this.state.location);
      case 'location.openSettings':
        if (this.state.permissions.some((row) => row.pending))
          return failure(id, 'conflict', 'Permission presentation is pending');
        if (this.state.location.permissionAction !== 'openSettings')
          return failure(id, 'invalid_state', 'Permission action changed');
        return success(id, this.state.location);
      case 'bluetooth.scan':
        this.change({
          ...this.state,
          bluetooth: { ...this.state.bluetooth, scanning: false },
          discoveredDevices: [discoveredBoard],
        });
        this.scheduleDiscoveryExpiry();
        return accepted(id);
      case 'device.connect': {
        const deviceId = string(params.deviceId);
        const trackingMode = params.trackingMode;
        if (!deviceId) return failure(id, 'invalid_request', 'deviceId is required');
        if (trackingMode !== 'saved' && trackingMode !== 'guest') {
          return failure(id, 'invalid_request', 'trackingMode must be saved or guest');
        }
        const active = this.state.activeConnection;
        if (active && (active.deviceId !== deviceId || active.state !== 'failed')) {
          return failure(id, 'invalid_state', 'A board is already active');
        }
        if (active && active.trackingMode !== trackingMode) {
          return failure(id, 'invalid_state', 'A retry must keep the active tracking mode');
        }
        if (!active && deviceId !== discoveredBoard.deviceId) {
          return failure(id, 'not_found', 'Device is no longer available');
        }
        const savedDeviceId =
          trackingMode === 'saved' ? (discoveredBoard.savedDeviceId ?? savedBoard.savedDeviceId) : null;
        const savedDevice = savedDeviceId
          ? this.state.savedDevices.find((value) => value.savedDeviceId === savedDeviceId)
          : undefined;
        if (trackingMode === 'saved' && !savedDevice) {
          return failure(id, 'not_found', 'Saved board is not available');
        }
        this.activeSessionId = trackingMode === 'saved' ? `session-${this.nextSession++}` : undefined;
        if (this.discoveryExpiryTimer) clearTimeout(this.discoveryExpiryTimer);
        this.discoveryExpiryTimer = undefined;
        const activeSession: SessionSummary | null = this.activeSessionId
          ? {
              ...session,
              sessionId: this.activeSessionId,
              deviceHistoryId: savedDevice?.deviceHistoryId ?? session.deviceHistoryId,
              displayName: savedDevice?.displayName ?? session.displayName,
              startedAt: new Date().toISOString(),
              endedAt: null,
              state: 'active' as const,
              sampleCount: 0,
              ridingElapsedMs: 0,
              totalElapsedMs: 0,
            }
          : null;
        if (activeSession) this.sessionCatalog = [activeSession, ...this.sessionCatalog];
        this.change(
          {
            ...this.state,
            discoveredDevices: [],
            activeConnection: {
              ...discoveredBoard,
              state: 'ready',
              deviceKind: 'board',
              trackingMode,
              savedDeviceId,
              displayName: savedDevice?.displayName ?? discoveredBoard.name,
              controllerConnection: { kind: 'direct' },
              controllerCandidates: [],
              connectionIssue: null,
              bms: { status: 'discovering', connection: null, candidates: [], latestSnapshot: null },
              refloatControls: readyRefloatControls(),
              refloatConfiguration: {
                availability: 'ready',
                writable: true,
                packageVersion: refloatIdentity.packageVersion,
                schemaFingerprint: refloatIdentity.schemaFingerprint,
                wireSignature: refloatIdentity.wireSignature,
                currentLoaded: true,
                defaultLoaded: false,
                pendingOperation: null,
              },
              controllerActions: { shutdownAvailable: false, pendingAction: null },
              activeSession,
              latestSample: null,
              batteryEstimate: emptyEstimate(),
              liveWidgetSlots: [...defaultLiveWidgetSlots],
              latestLocation: null,
              currentLimits: {
                motorCurrentMaxA: 100,
                motorCurrentMinA: -100,
                batteryCurrentMaxA: 50,
                batteryCurrentMinA: -20,
                batterySeriesCells: 18,
              },
              firmware: {
                vescVersion: '6.06',
                packageName: 'refloat',
                packageVersion: '1.3.0',
                hardwareName: 'Express',
                accessMode: 'readWrite',
                supported: true,
              },
            },
          },
          Boolean(activeSession)
        );
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
        if (active.trackingMode === 'saved') {
          const profile = this.profiles.get(active.savedDeviceId ?? '');
          if (!profile) return failure(id, 'invalid_state', 'Saved board is unavailable');
          const updated = {
            ...profile,
            revision: profile.revision + 1,
            values: { ...profile.values, preferredCanId: canId },
          };
          this.profiles.set(profile.savedDeviceId, updated);
        }
        this.pendingCandidates = [];
        this.change({
          ...this.state,
          activeConnection: {
            ...active,
            state: 'identifying',
            controllerConnection: { kind: 'can', canId },
            controllerCandidates: [],
            connectionIssue: null,
          },
        });
        return accepted(id);
      }
      case 'savedDevice.update': {
        const savedDeviceId = string(params.savedDeviceId);
        const expectedRevision = number(params.expectedRevision);
        const current = this.state.savedDevices.find((device) => device.savedDeviceId === savedDeviceId);
        const name = params.customDisplayName;
        if (!current || expectedRevision === null || (name !== null && typeof name !== 'string')) {
          return failure(id, 'invalid_request', 'Valid saved device update parameters are required');
        }
        if (current.revision !== expectedRevision) {
          return failure(id, 'conflict', 'Saved device revision changed');
        }
        const customDisplayName = typeof name === 'string' && name.trim() ? name.trim() : null;
        const updated = {
          ...current,
          revision: current.revision + 1,
          customDisplayName,
          displayName: customDisplayName ?? current.advertisedName ?? current.bluetoothIdentifier ?? 'Board',
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
                  displayName: updated.displayName,
                }
              : this.state.activeConnection,
        });
        return success(id, updated);
      }
      case 'savedDevice.delete': {
        const savedDeviceId = string(params.savedDeviceId);
        const current = savedDeviceId
          ? this.state.savedDevices.find((device) => device.savedDeviceId === savedDeviceId)
          : undefined;
        if (!current) return accepted(id);
        if (this.state.activeConnection?.savedDeviceId === savedDeviceId) {
          return failure(id, 'invalid_state', 'Disconnect the board before forgetting it');
        }
        this.sessionCatalog = this.sessionCatalog.filter((item) => item.deviceHistoryId !== current.deviceHistoryId);
        this.profiles.delete(current.savedDeviceId);
        this.change(
          {
            ...this.state,
            savedDevices: this.state.savedDevices.filter((device) => device.savedDeviceId !== savedDeviceId),
          },
          true
        );
        return accepted(id);
      }
      case 'device.disconnect': {
        const activeSessionId = this.activeSessionId;
        this.activeSessionId = undefined;
        if (activeSessionId) {
          this.sessionCatalog = this.sessionCatalog.map((item) =>
            item.sessionId === activeSessionId
              ? { ...item, endedAt: new Date().toISOString(), state: 'completed' }
              : item
          );
        }
        this.change(
          {
            ...this.state,
            activeConnection: null,
          },
          Boolean(activeSessionId)
        );
        return accepted(id);
      }
      case 'controller.shutdown':
        if (Object.keys(params).length !== 0) {
          return failure(id, 'invalid_request', 'Controller shutdown takes no parameters');
        }
        return this.shutdownController(id);
      case 'refloat.control.set': {
        if (
          Object.keys(params).sort().join(',') !== 'control,enabled' ||
          !isRefloatControl(params.control) ||
          typeof params.enabled !== 'boolean'
        ) {
          return failure(id, 'invalid_request', 'A supported control and boolean enabled value are required');
        }
        return this.setRefloatControl(id, params.control, params.enabled);
      }
      case 'refloat.fatalError.clear':
        if (Object.keys(params).length !== 0) {
          return failure(id, 'invalid_request', 'Fatal error clear takes no parameters');
        }
        return this.clearRefloatFatalError(id);
      case 'refloat.configuration.open': {
        if (Object.keys(params).length !== 0)
          return failure(id, 'invalid_request', 'Configuration open takes no parameters');
        const summary = this.state.activeConnection?.refloatConfiguration;
        return summary ? success(id, summary) : failure(id, 'invalid_state', 'Refloat configuration is unavailable');
      }
      case 'refloat.configuration.schema': {
        if (Object.keys(params).some((key) => key !== 'cursor')) {
          return failure(id, 'invalid_request', 'Configuration schema parameters are invalid');
        }
        const summary = this.state.activeConnection?.refloatConfiguration;
        if (!summary || summary.availability !== 'ready') return failure(id, 'invalid_state', 'Schema is unavailable');
        const page = decodeRefloatPageCursor(params.cursor);
        if (page === null || page > 1) return failure(id, 'invalid_request', 'Schema cursor is invalid');
        return success(id, {
          identity: refloatIdentity,
          group: page === 0 ? 'General' : 'Lighting',
          subgroup: page === 0 ? 'Balance' : 'Startup defaults',
          fields: page === 0 ? refloatFields.slice(0, 1) : refloatFields.slice(1),
          nextCursor: page === 0 ? encodeRefloatPageCursor(1) : null,
        });
      }
      case 'refloat.configuration.read': {
        if (
          Object.keys(params).join(',') !== 'source' ||
          (params.source !== 'current' && params.source !== 'default')
        ) {
          return failure(id, 'invalid_request', 'Configuration source must be current or default');
        }
        const active = this.state.activeConnection;
        const summary = active?.refloatConfiguration;
        if (!active || !summary || summary.availability !== 'ready')
          return failure(id, 'invalid_state', 'Configuration is unavailable');
        if (params.source === 'current' && active.refloatControls?.flywheelEnabled) {
          return failure(id, 'invalid_state', 'Current configuration is unavailable in flywheel mode');
        }
        const source = params.source;
        const values = source === 'current' ? this.refloatCurrentValues : defaultRefloatValues;
        this.change({
          ...this.state,
          activeConnection: {
            ...active,
            refloatConfiguration: {
              ...summary,
              currentLoaded: source === 'current' || summary.currentLoaded,
              defaultLoaded: source === 'default' || summary.defaultLoaded,
              writable: source === 'current' ? active.firmware?.accessMode === 'readWrite' : summary.writable,
            },
          },
        });
        return success(id, {
          identity: refloatIdentity,
          revision: source === 'current' ? this.refloatCurrentRevision : 1,
          source,
          values,
        });
      }
      case 'refloat.configuration.write':
        return this.writeRefloatConfiguration(id, params);
      case 'refloat.tune.list':
        return this.listRefloatTunes(id, params);
      case 'refloat.tune.get': {
        const tune = string(params.tuneId) ? this.tunes.get(String(params.tuneId)) : undefined;
        return tune ? success(id, tune) : failure(id, 'not_found', 'Tune was not found');
      }
      case 'refloat.tune.create':
        return this.createRefloatTune(id, params);
      case 'refloat.tune.update':
        return this.updateRefloatTune(id, params);
      case 'refloat.tune.delete':
        return this.deleteRefloatTune(id, params);
      case 'refloat.tune.apply': {
        const tune = string(params.tuneId) ? this.tunes.get(String(params.tuneId)) : undefined;
        if (!tune) return failure(id, 'not_found', 'Tune was not found');
        if (integer(params.expectedRevision) !== tune.revision) return failure(id, 'conflict', 'Tune revision changed');
        return this.writeRefloatConfiguration(id, {
          packageVersion: tune.identity.packageVersion,
          schemaFingerprint: tune.identity.schemaFingerprint,
          wireSignature: tune.identity.wireSignature,
          baseRevision: params.baseRevision,
          values: tune.values,
        });
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
          'minimumSessionDistance',
          'themePreference',
          'unitPreference',
          'liveWidgetSlots',
          'sessionDisplayOptions',
          'historyGapThresholdSeconds',
          'locationRecordingEnabled',
          'includeGpsLocationWhenExporting',
        ]);
        const historyGap = patch.historyGapThresholdSeconds;
        if (
          !Object.keys(patch).length ||
          Object.keys(patch).some((key) => !allowed.has(key)) ||
          ('themePreference' in patch && !['system', 'light', 'dark'].includes(String(patch.themePreference))) ||
          ('sessionDisplayOptions' in patch &&
            (!patch.sessionDisplayOptions ||
              Object.keys(patch.sessionDisplayOptions).length !== 2 ||
              typeof (patch.sessionDisplayOptions as Record<string, unknown>).bothFootpadsOff !== 'boolean' ||
              typeof (patch.sessionDisplayOptions as Record<string, unknown>).oneFootpadOff !== 'boolean')) ||
          ('liveWidgetSlots' in patch &&
            (!Array.isArray(patch.liveWidgetSlots) ||
              patch.liveWidgetSlots.length !== 5 ||
              patch.liveWidgetSlots.some((field) => !liveWidgetFields.includes(field)))) ||
          ('unitPreference' in patch && !['metric', 'imperial'].includes(String(patch.unitPreference))) ||
          ('historyGapThresholdSeconds' in patch &&
            historyGap !== null &&
            (typeof historyGap !== 'number' ||
              !Number.isFinite(historyGap) ||
              historyGap < 0.1 ||
              historyGap > 3_600)) ||
          ('locationRecordingEnabled' in patch && typeof patch.locationRecordingEnabled !== 'boolean') ||
          ('includeGpsLocationWhenExporting' in patch && typeof patch.includeGpsLocationWhenExporting !== 'boolean')
        ) {
          return failure(id, 'invalid_request', 'Settings patch is invalid');
        }
        const settings = {
          ...this.state.settings,
          ...patch,
          revision: this.state.settings.revision + 1,
        };
        const location = locationSummary(this.state.permissions, settings.locationRecordingEnabled);
        this.change({ ...this.state, settings, location });
        return success(id, settings);
      }
      case 'boardProfile.get': {
        const profile = this.profiles.get(string(params.savedDeviceId) ?? '');
        return profile ? success(id, profile) : failure(id, 'not_found', 'Board Profile was not found');
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
        if (
          'preferredBmsCanId' in patch &&
          patch.preferredBmsCanId !== null &&
          (integer(patch.preferredBmsCanId) === null ||
            (integer(patch.preferredBmsCanId) ?? -1) < 0 ||
            (integer(patch.preferredBmsCanId) ?? 254) > 253)
        ) {
          return failure(id, 'invalid_request', 'Preferred BMS CAN ID must be 0 through 253');
        }
        if (
          'liveWidgetSlots' in patch &&
          (!Array.isArray(patch.liveWidgetSlots) ||
            patch.liveWidgetSlots.length !== 5 ||
            patch.liveWidgetSlots.some((field) => !liveWidgetFields.includes(field)))
        )
          return failure(id, 'invalid_request', 'Invalid live box layout');
        if (patch.batteryCellProfileId != null && !batteryCellProfiles.some((p) => p.id === patch.batteryCellProfileId))
          return failure(id, 'invalid_request', 'Unknown battery cell profile');
        if (
          patch.batteryMinimumCellVoltageV != null &&
          (typeof patch.batteryMinimumCellVoltageV !== 'number' ||
            !Number.isFinite(patch.batteryMinimumCellVoltageV) ||
            patch.batteryMinimumCellVoltageV < 2.7 ||
            patch.batteryMinimumCellVoltageV >= 4.2)
        )
          return failure(id, 'invalid_request', 'Empty voltage must be from 2.7 V to below 4.2 V');
        let values;
        try {
          values = applyBatteryPatch(current.values, patch);
        } catch {
          return failure(id, 'invalid_request', 'Invalid battery configuration');
        }
        const updated = {
          ...current,
          revision: current.revision + 1,
          values,
          missingFields: ['seriesCount', 'parallelCount', 'motor'].filter((field) => values[field] == null),
        };
        this.profiles.set(savedDeviceId, updated);
        this.change({
          ...this.state,
          savedDevices: this.state.savedDevices.map((device) =>
            device.savedDeviceId === savedDeviceId
              ? { ...device, profileComplete: updated.missingFields.length === 0 }
              : device
          ),
        });
        return success(id, updated);
      }
      case 'session.marker.add': {
        const sessionId = string(params.sessionId);
        const operationId = string(params.operationId);
        const found = this.sessionCatalog.find((s) => s.sessionId === sessionId);
        if (!found || sessionId !== this.activeSessionId)
          return failure(id, 'invalid_state', 'No active saved session');
        if (!operationId || !/^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(operationId))
          return failure(id, 'invalid_request', 'Invalid operation ID');
        const rows = this.rideMarkers.get(sessionId) ?? [];
        let marker = rows.find((m) => m.operationId === operationId);
        if (!marker) {
          const elapsedMs = Math.max(found.totalElapsedMs, Date.now() - Date.parse(found.startedAt));
          if (rows.length >= 1000 || (rows.length && elapsedMs - rows.at(-1)!.elapsedMs < 500))
            return failure(id, 'invalid_request', 'Marker limit reached');
          marker = { sequence: found.eventCount, elapsedMs, observedAtMs: Date.now(), operationId, location: null };
          rows.push(marker);
          this.rideMarkers.set(sessionId, rows);
          found.eventCount++;
          found.totalElapsedMs = elapsedMs;
          this.change({
            ...this.state,
            activeConnection: this.state.activeConnection
              ? { ...this.state.activeConnection, activeSession: found }
              : null,
          });
        }
        return success(id, { marker, snapshot: sessionMetadata(found).snapshot });
      }
      case 'session.get': {
        const found = this.findSession(params);
        return found
          ? success(id, { ...sessionMetadata(found), markerCount: this.rideMarkers.get(found.sessionId)?.length ?? 0 })
          : failure(id, 'not_found', 'Session was not found');
      }
      case 'session.list': {
        const savedDeviceId = string(params.savedDeviceId);
        const limit = integer(params.limit);
        if (!savedDeviceId) {
          return failure(id, 'invalid_request', 'savedDeviceId is required');
        }
        if (limit === null || limit < 1 || limit > MAX_SESSION_LIST_PAGE) {
          return failure(id, 'invalid_request', 'Session page size must be 1 through 100');
        }
        const board = this.state.savedDevices.find((value) => value.savedDeviceId === savedDeviceId);
        if (!board) return failure(id, 'not_found', 'Saved board was not found');
        const page = this.sessionList(board.deviceHistoryId, params.cursor, limit);
        return page.error ? failure(id, page.error.code, page.error.message) : success(id, page.value);
      }
      case 'session.export':
        return this.findSession(params)
          ? success(id, {
              fileName: `${string(params.sessionId)}.${params.format === 'csv' ? 'csv' : 'floatview'}`,
              delivery: 'staged',
            })
          : failure(id, 'not_found', 'Session was not found');
      case 'session.delete': {
        const found = this.findSession(params);
        if (!found) return accepted(id);
        this.sessionCatalog = this.sessionCatalog.filter((item) => item.sessionId !== found.sessionId);
        this.rideMarkers.delete(found.sessionId);
        this.change(
          {
            ...this.state,
          },
          true
        );
        return accepted(id);
      }
      case 'session.delete.cancel':
        return this.operation?.operationId === string(params.operationId)
          ? success(id, this.operation)
          : failure(id, 'not_found', 'Deletion operation was not found');
      case 'session.prune.preview': {
        const target = integer(params.targetResolutionSeconds);
        if (!target || target < 1 || target > 300)
          return failure(id, 'invalid_request', 'Prune resolution must be 1 through 300 seconds');
        const found = params.scope === 'one' ? this.findSession(params) : undefined;
        if (params.scope === 'one' && !found) return failure(id, 'not_found', 'Session was not found');
        const operation = this.startOperation(
          'prunePreview',
          found?.sessionId ?? null,
          params.scope === 'all' ? 'all' : 'one',
          target
        );
        const preview: SessionPrunePreview = {
          operation,
          recordedSampleCount:
            found?.sampleCount ?? this.sessionCatalog.reduce((sum, item) => sum + item.sampleCount, 0),
          estimatedStoredSampleCount: found
            ? Math.min(found.sampleCount, Math.ceil(found.totalElapsedMs / (target * 1000)) * 8 + 2)
            : 0,
          estimatedReclaimableBytes: found ? Math.max(0, found.sampleCount - 2) * 256 : null,
          skipped: false,
          skipReason: null,
        };
        return success(id, preview);
      }
      case 'session.prune.start': {
        const target = integer(params.targetResolutionSeconds);
        if (!target || target < 1 || target > 300)
          return failure(id, 'invalid_request', 'Prune resolution must be 1 through 300 seconds');
        const found = params.scope === 'one' ? this.findSession(params) : undefined;
        if (params.scope === 'one' && !found) return failure(id, 'not_found', 'Session was not found');
        return success(
          id,
          this.startOperation('prune', found?.sessionId ?? null, params.scope === 'all' ? 'all' : 'one', target)
        );
      }
      case 'session.prune.cancel':
        return this.operation?.operationId === string(params.operationId)
          ? success(id, this.cancelOperation())
          : failure(id, 'not_found', 'Prune operation was not found');
      default:
        return failure(id, 'invalid_request', 'Unknown method');
    }
  }

  publishTelemetry(): void {
    const active = this.state.activeConnection;
    if (!active || active.state !== 'ready') return;
    const sequence = (active.latestSample?.sequence ?? 0) + 1;
    const elapsedMs = sequence * this.state.settings.ridingPersistIntervalMs;
    const latestSample = {
      sequence,
      observedAt: new Date().toISOString(),
      slowObservedAt: new Date(Date.now() - 250).toISOString(),
      elapsedMs,
      rideState: 'riding' as const,
      packageState: 'running' as const,
      packageMode: 'normal' as const,
      footpadState: 'both' as const,
      stopCondition: 'none' as const,
      setpointAdjustment: 'none' as const,
      wheelslip: false,
      darkride: false,
      charging: false,
      alertReason: 'none' as const,
      firmwareFault: null,
      packVoltageV: 58.4 - sequence * 0.002,
      dutyCycle: 0.42,
      controllerTempC: 38.2,
      motorTempC: 41.7,
      motorCurrentA: 18.6,
      batteryCurrentA: 8.4,
      erpm: 3240,
      speedKph: 24.3,
      pitchDeg: 1.4,
      balancePitchDeg: 1.1,
      rollDeg: 0.6,
      directionalMotorCurrentA: 17.9,
      filteredMotorCurrentA: 17.4,
      requestedBalanceCurrentA: 20.2,
      boosterCurrentA: 1.2,
      fieldWeakeningCurrentA: 0,
      footpadAdc1V: 2.8,
      footpadAdc2V: 2.75,
      remoteInput: 0,
      setpointDeg: 1,
      atrSetpointDeg: 0.4,
      brakeTiltSetpointDeg: 0,
      torqueTiltSetpointDeg: 0.6,
      turnTiltSetpointDeg: 0,
      remoteSetpointDeg: 0,
      atrAccelerationDifferenceErpmPerSecond: 180,
      atrSpeedBoost: 0.05,
      distanceM: sequence * 0.675,
      odometerM: 12_345 + sequence * 0.675,
      ampHoursDrawnAh: 18.4,
      ampHoursChargedAh: 2.7,
      wattHoursDrawnWh: 812,
      wattHoursChargedWh: 119,
      batteryLevel: 0.68,
      chargingCurrentA: 0,
      chargingVoltageV: 0,
      extensions: {},
    };
    const activeSession = active.activeSession
      ? {
          ...active.activeSession,
          sampleCount: sequence,
          ridingElapsedMs: elapsedMs,
          totalElapsedMs: elapsedMs,
          maxSpeedMps: latestSample.speedKph / 3.6,
          maxDutyCycle: latestSample.dutyCycle,
        }
      : null;
    if (activeSession) {
      this.sessionCatalog = this.sessionCatalog.map((item) =>
        item.sessionId === activeSession.sessionId ? activeSession : item
      );
    }
    const latestLocation =
      active.trackingMode === 'saved'
        ? {
            latitude: -33.8688,
            longitude: 151.2093,
            accuracyM: 12,
            observedAt: latestSample.observedAt,
          }
        : null;
    this.change({
      ...this.state,
      activeConnection: {
        ...active,
        latestSample,
        latestLocation,
        activeSession,
        refloatControls: active.refloatControls ? { ...active.refloatControls, fatalErrorClearable: false } : null,
        controllerActions: { shutdownAvailable: false, pendingAction: null },
      },
    });
  }

  get telemetryCadenceMs(): number {
    return this.state.settings.ridingPersistIntervalMs;
  }

  simulatePermissions(permissions: AppState['permissions']): void {
    this.change({
      ...this.state,
      permissions,
      location: locationSummary(permissions, this.state.settings.locationRecordingEnabled),
    });
  }

  simulateConnectionState(state: 'ready' | 'reconnecting'): void {
    const active = this.state.activeConnection;
    if (!active) return;
    const refloatControls =
      state === 'reconnecting'
        ? {
            status: 'loading' as const,
            writable: false,
            packageEnabled: null,
            flywheelEnabled: null,
            lightingAvailable: false,
            ledsEnabled: null,
            headlightsEnabled: null,
            fatalErrorClearAvailable: false,
            fatalErrorLatched: null,
            fatalErrorActive: null,
            fatalErrorClearable: null,
            pendingAction: null,
          }
        : readyRefloatControls();
    this.change({
      ...this.state,
      activeConnection: {
        ...active,
        state,
        refloatControls,
        controllerActions: {
          shutdownAvailable: shutdownAvailable({ ...active, state, refloatControls }),
          pendingAction: null,
        },
      },
    });
  }

  simulateRefloatControls(patch: Partial<RefloatControls>): void {
    const active = this.state.activeConnection;
    if (!active?.refloatControls) return;
    const refloatControls = { ...active.refloatControls, ...patch };
    this.change({
      ...this.state,
      activeConnection: {
        ...active,
        refloatControls,
        controllerActions: {
          shutdownAvailable: shutdownAvailable({ ...active, refloatControls }),
          pendingAction: active.controllerActions.pendingAction,
        },
      },
    });
  }

  simulateRefloatTimeout(): void {
    this.refloatTimeoutOnce = true;
  }

  simulateShutdownOutcome(outcome: 'no-op' | 'disconnect'): void {
    this.shutdownOutcome = outcome;
  }

  simulateRideState(rideState: 'idle' | 'riding' | 'unknown'): void {
    const active = this.state.activeConnection;
    if (!active?.latestSample) return;
    const latestSample = {
      ...active.latestSample,
      rideState,
      ...(rideState === 'idle'
        ? { footpadState: 'none' as const, packageState: 'ready' as const, packageMode: 'normal' as const }
        : {}),
    };
    const available = shutdownAvailable({ ...active, latestSample });
    this.change({
      ...this.state,
      activeConnection: {
        ...active,
        latestSample,
        controllerActions: { shutdownAvailable: available, pendingAction: null },
      },
    });
  }

  simulateExternalLighting(ledsEnabled: boolean, headlightsEnabled: boolean): void {
    const controls = this.state.activeConnection?.refloatControls;
    if (!controls || !controls.lightingAvailable) return;
    if (controls.ledsEnabled === ledsEnabled && controls.headlightsEnabled === headlightsEnabled) return;
    this.simulateRefloatControls({ ledsEnabled, headlightsEnabled });
  }

  simulateBmsState(bms: BmsState): void {
    const active = this.state.activeConnection;
    if (!active || !validBmsState(bms)) return;
    this.change({ ...this.state, activeConnection: { ...active, bms } });
  }

  simulateBmsAttached(snapshot: BmsSnapshot, connection: BmsState['connection'] = { kind: 'direct' }): void {
    if (!connection) return;
    this.simulateBmsState({ status: 'attached', connection, candidates: [], latestSnapshot: snapshot });
  }

  simulateStandaloneBms(snapshot: BmsSnapshot): void {
    const active = this.state.activeConnection;
    if (!active) return;
    const activeSessionId = active.activeSession?.sessionId;
    this.activeSessionId = undefined;
    if (activeSessionId) this.sessionCatalog = this.sessionCatalog.filter((item) => item.sessionId !== activeSessionId);
    this.change(
      {
        ...this.state,
        activeConnection: {
          ...active,
          state: 'ready',
          deviceKind: 'bms',
          firmware: null,
          controllerConnection: null,
          controllerCandidates: [],
          connectionIssue: null,
          bms: { status: 'attached', connection: { kind: 'direct' }, candidates: [], latestSnapshot: snapshot },
          refloatControls: null,
          refloatConfiguration: null,
          controllerActions: { shutdownAvailable: false, pendingAction: null },
          activeSession: null,
          latestSample: null,
          batteryEstimate: emptyEstimate(),
          liveWidgetSlots: [...defaultLiveWidgetSlots],
          latestLocation: null,
          currentLimits: null,
        },
      },
      Boolean(activeSessionId)
    );
  }

  simulateBmsAmbiguous(candidates: BmsCandidate[]): void {
    this.simulateBmsState({
      status: 'ambiguous',
      connection: null,
      candidates: [...candidates].sort(compareBmsCandidates),
      latestSnapshot: null,
    });
  }

  simulateIdentifying(): void {
    const active = this.state.activeConnection;
    if (!active) return;
    const activeSessionId = active.activeSession?.sessionId;
    this.activeSessionId = undefined;
    if (activeSessionId) this.sessionCatalog = this.sessionCatalog.filter((item) => item.sessionId !== activeSessionId);
    this.change(
      {
        ...this.state,
        activeConnection: {
          ...active,
          state: 'identifying',
          savedDeviceId: active.trackingMode === 'saved' ? active.savedDeviceId : null,
          controllerConnection: null,
          controllerCandidates: [],
          connectionIssue: null,
          activeSession: null,
          firmware: null,
        },
      },
      Boolean(activeSessionId)
    );
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
        connectionIssue: null,
      },
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
        activeSession: null,
      },
    });
  }

  simulateConnectionFailure(code: ConnectionIssueCode, message: string): void {
    const active = this.state.activeConnection;
    if (!active) return;
    const activeSessionId = active.activeSession?.sessionId;
    this.activeSessionId = undefined;
    if (activeSessionId) {
      this.sessionCatalog = this.sessionCatalog
        .map((item) =>
          item.sessionId === activeSessionId
            ? { ...item, endedAt: new Date().toISOString(), state: 'interrupted' as const, integrity: 'gaps' as const }
            : item
        )
        .filter((item) => code === 'controller_changed' || item.sessionId !== activeSessionId);
    }
    this.change(
      {
        ...this.state,
        activeConnection: {
          ...active,
          state: 'failed',
          connectionIssue: { code, message: message.slice(0, 1_024) },
          controllerCandidates: [],
          activeSession: null,
        },
      },
      Boolean(activeSessionId)
    );
  }

  private findSession(params: Record<string, unknown>) {
    const sessionId = string(params.sessionId);
    return this.sessionCatalog.find((item) => item.sessionId === sessionId);
  }

  private writeRefloatConfiguration(id: number, params: Record<string, unknown>): Response {
    const active = this.state.activeConnection;
    const summary = active?.refloatConfiguration;
    const values = asObject(params.values);
    if (
      typeof params.packageVersion !== 'string' ||
      typeof params.schemaFingerprint !== 'string' ||
      integer(params.wireSignature) === null ||
      integer(params.baseRevision) === null ||
      !values ||
      !validRefloatValues(values)
    )
      return failure(id, 'invalid_request', 'Complete typed configuration values are required');
    if (
      !active ||
      !summary ||
      !summary.writable ||
      active.state !== 'ready' ||
      active.refloatControls?.flywheelEnabled ||
      active.refloatControls?.pendingAction !== null ||
      active.controllerActions.pendingAction !== null ||
      active.latestSample?.rideState !== 'idle' ||
      active.latestSample.footpadState !== 'none' ||
      active.latestSample.packageMode !== 'normal'
    )
      return failure(id, 'invalid_state', 'Configuration write is unavailable');
    if (
      params.packageVersion !== refloatIdentity.packageVersion ||
      params.schemaFingerprint !== refloatIdentity.schemaFingerprint ||
      params.wireSignature !== refloatIdentity.wireSignature ||
      params.baseRevision !== this.refloatCurrentRevision
    )
      return failure(id, 'conflict', 'Configuration identity or base revision changed');
    this.refloatCurrentRevision += 1;
    this.refloatCurrentValues = {
      ...(values as Record<string, RefloatConfigurationValue>),
      'meta.is_default': { type: 'boolean', value: false },
    };
    this.change({
      ...this.state,
      activeConnection: {
        ...active,
        refloatConfiguration: { ...summary, currentLoaded: true, pendingOperation: null },
      },
    });
    return success(id, {
      identity: refloatIdentity,
      revision: this.refloatCurrentRevision,
      source: 'current',
      values: this.refloatCurrentValues,
    });
  }

  private listRefloatTunes(id: number, params: Record<string, unknown>): Response {
    const identity = validRefloatIdentity(params.identity);
    const limit = integer(params.limit);
    const offset = decodeTuneCursor(params.cursor);
    if (!identity || !limit || limit > MAX_REFLOAT_TUNE_LIST_PAGE || offset === null) {
      return failure(id, 'invalid_request', 'Tune list parameters are invalid');
    }
    const matching = [...this.tunes.values()]
      .filter((tune) => sameRefloatIdentity(tune.identity, identity))
      .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt) || left.tuneId.localeCompare(right.tuneId));
    const items = matching.slice(offset, offset + limit).map((tune) => ({
      tuneId: tune.tuneId,
      revision: tune.revision,
      name: tune.name,
      description: tune.description,
      identity: tune.identity,
      createdAt: tune.createdAt,
      updatedAt: tune.updatedAt,
    }));
    return success(id, {
      items,
      nextCursor: offset + limit < matching.length ? encodeTuneCursor(offset + limit) : null,
    });
  }

  private createRefloatTune(id: number, params: Record<string, unknown>): Response {
    const name = boundedText(params.name, MAX_REFLOAT_TUNE_NAME_BYTES, false);
    const description = boundedText(params.description, MAX_REFLOAT_TUNE_DESCRIPTION_BYTES, true);
    const identity = validRefloatIdentity(params.identity);
    const values = asObject(params.values);
    if (
      typeof name !== 'string' ||
      description === undefined ||
      !identity ||
      !sameRefloatIdentity(identity, refloatIdentity) ||
      integer(params.sourceRevision) !== this.refloatCurrentRevision ||
      !values ||
      !validRefloatValues(values)
    )
      return failure(id, 'invalid_request', 'Tune create parameters are invalid or stale');
    const now = new Date().toISOString();
    const tune: RefloatTune = {
      tuneId: `tune-${this.nextTune++}`,
      revision: 1,
      name,
      description,
      identity,
      values: values as Record<string, RefloatConfigurationValue>,
      createdAt: now,
      updatedAt: now,
    };
    this.tunes.set(tune.tuneId, tune);
    return success(id, tune);
  }

  private updateRefloatTune(id: number, params: Record<string, unknown>): Response {
    const tuneId = string(params.tuneId);
    const current = tuneId ? this.tunes.get(tuneId) : undefined;
    if (!current) return failure(id, 'not_found', 'Tune was not found');
    if (integer(params.expectedRevision) !== current.revision) return failure(id, 'conflict', 'Tune revision changed');
    const name = boundedText(params.name, MAX_REFLOAT_TUNE_NAME_BYTES, false);
    const description = boundedText(params.description, MAX_REFLOAT_TUNE_DESCRIPTION_BYTES, true);
    const values = asObject(params.values);
    if (typeof name !== 'string' || description === undefined || !values || !validRefloatValues(values)) {
      return failure(id, 'invalid_request', 'Tune update parameters are invalid');
    }
    const updated = {
      ...current,
      revision: current.revision + 1,
      name,
      description,
      values: values as Record<string, RefloatConfigurationValue>,
      updatedAt: new Date().toISOString(),
    };
    this.tunes.set(tuneId!, updated);
    return success(id, updated);
  }

  private deleteRefloatTune(id: number, params: Record<string, unknown>): Response {
    const tuneId = string(params.tuneId);
    const current = tuneId ? this.tunes.get(tuneId) : undefined;
    if (!current) return accepted(id);
    if (integer(params.expectedRevision) !== current.revision) return failure(id, 'conflict', 'Tune revision changed');
    this.tunes.delete(tuneId!);
    return accepted(id);
  }

  private setRefloatControl(id: number, control: RefloatControl, enabled: boolean): Response {
    const active = this.state.activeConnection;
    const controls = active?.refloatControls;
    if (!active || active.state !== 'ready' || !controls || controls.status !== 'ready' || !controls.writable) {
      return failure(id, 'invalid_state', 'Refloat controls are not writable');
    }
    if (controls.pendingAction !== null) return failure(id, 'invalid_state', 'Another Refloat action is pending');
    if (controls.flywheelEnabled && !(control === 'flywheel' && !enabled)) {
      return failure(id, 'invalid_state', 'Only flywheel disable is available in flywheel mode');
    }
    if ((control === 'leds' || control === 'headlights') && !controls.lightingAvailable) {
      return failure(id, 'invalid_state', 'Refloat lighting is unavailable');
    }
    const activeFlywheelDisable = control === 'flywheel' && !enabled && controls.flywheelEnabled === true;
    if (
      (control === 'package' || (control === 'flywheel' && !activeFlywheelDisable)) &&
      active.latestSample?.rideState === 'riding'
    ) {
      return failure(id, 'invalid_state', 'The board must be idle for this control');
    }
    const field = refloatControlField(control);
    if (controls[field] === null) return failure(id, 'invalid_state', 'The control state is unavailable');
    if (controls[field] === enabled) return success(id, controls);
    const result = this.completeRefloatAction(id, control, { [field]: enabled });
    if (control === 'flywheel' && 'result' in result) {
      const completed = this.state.activeConnection;
      const summary = completed?.refloatConfiguration;
      if (completed && summary) {
        if (enabled) {
          this.change({
            ...this.state,
            activeConnection: {
              ...completed,
              refloatConfiguration: { ...summary, currentLoaded: false, writable: false, pendingOperation: null },
            },
          });
        } else {
          this.change({
            ...this.state,
            activeConnection: {
              ...completed,
              refloatConfiguration: { ...summary, currentLoaded: true, writable: true, pendingOperation: null },
            },
          });
        }
      }
    }
    return result;
  }

  private clearRefloatFatalError(id: number): Response {
    const active = this.state.activeConnection;
    const controls = active?.refloatControls;
    if (
      !active ||
      active.state !== 'ready' ||
      !controls ||
      controls.status !== 'ready' ||
      !controls.writable ||
      controls.pendingAction !== null ||
      controls.flywheelEnabled ||
      !controls.fatalErrorClearAvailable ||
      controls.fatalErrorLatched !== true ||
      controls.fatalErrorActive !== false ||
      controls.fatalErrorClearable !== true ||
      active.latestSample?.rideState === 'riding'
    ) {
      return failure(id, 'invalid_state', 'The fatal error is not clearable');
    }
    return this.completeRefloatAction(id, 'clearFatalError', {
      fatalErrorLatched: false,
      fatalErrorClearable: false,
    });
  }

  private shutdownController(id: number): Response {
    const active = this.state.activeConnection;
    const actions = active?.controllerActions;
    if (
      !active ||
      active.state !== 'ready' ||
      !actions?.shutdownAvailable ||
      actions.pendingAction !== null ||
      active.latestSample?.rideState !== 'idle' ||
      active.refloatControls?.pendingAction !== null ||
      active.refloatControls?.flywheelEnabled === true
    ) {
      return failure(id, 'invalid_state', 'Controller shutdown is unavailable');
    }
    this.change({
      ...this.state,
      activeConnection: {
        ...active,
        controllerActions: { shutdownAvailable: false, pendingAction: 'shutdown' },
      },
    });
    if (this.shutdownOutcome === 'disconnect') {
      this.shutdownOutcome = 'no-op';
      this.change({ ...this.state, activeConnection: null });
      return accepted(id);
    }
    const pending = this.state.activeConnection!;
    this.change({
      ...this.state,
      activeConnection: {
        ...pending,
        controllerActions: { shutdownAvailable: true, pendingAction: null },
      },
    });
    return accepted(id);
  }

  private completeRefloatAction(
    id: number,
    action: RefloatPendingAction,
    confirmed: Partial<RefloatControls>
  ): Response {
    const active = this.state.activeConnection!;
    const controls = active.refloatControls!;
    this.change({
      ...this.state,
      activeConnection: { ...active, refloatControls: { ...controls, pendingAction: action } },
    });
    const pendingActive = this.state.activeConnection!;
    const pendingControls = pendingActive.refloatControls!;
    if (this.refloatTimeoutOnce) {
      this.refloatTimeoutOnce = false;
      const unchanged = { ...pendingControls, pendingAction: null };
      this.change({
        ...this.state,
        activeConnection: { ...pendingActive, refloatControls: unchanged },
      });
      return failure(id, 'unavailable', 'Refloat control confirmation timed out');
    }
    const complete = { ...pendingControls, ...confirmed, pendingAction: null };
    this.change({
      ...this.state,
      activeConnection: { ...pendingActive, refloatControls: complete },
    });
    return success(id, complete);
  }

  hasSession(sessionId: string): boolean {
    return Boolean(this.findSession({ sessionId }));
  }

  archiveIdentity(sessionId: string): { deviceHistoryId: string; displayName: string } | null {
    const found = this.findSession({ sessionId });
    return found ? { deviceHistoryId: found.deviceHistoryId, displayName: found.displayName } : null;
  }

  importSession(archive: Uint8Array): string {
    const imported = parseArchiveIdentity(archive);
    let importedBoard = this.state.savedDevices.find((value) => value.deviceHistoryId === imported.deviceHistoryId);
    if (!importedBoard) {
      importedBoard = {
        savedDeviceId: `saved-${imported.deviceHistoryId}`,
        deviceHistoryId: imported.deviceHistoryId,
        revision: 0,
        displayName: imported.displayName,
        customDisplayName: null,
        advertisedName: imported.displayName,
        bluetoothIdentifier: null,
        lastSeenAt: null,
        lastConnectedAt: null,
        profileComplete: false,
        summary: emptySummary(),
      };
      this.state = { ...this.state, savedDevices: [...this.state.savedDevices, importedBoard] };
      this.profiles.set(importedBoard.savedDeviceId, {
        savedDeviceId: importedBoard.savedDeviceId,
        revision: 0,
        values: {},
        missingFields: ['seriesCount', 'parallelCount', 'motor'],
      });
    }
    const sessionId = `imported-${this.nextSession++}`;
    const startedAt = new Date().toISOString();
    this.sessionCatalog = [
      {
        ...session,
        sessionId,
        deviceHistoryId: importedBoard.deviceHistoryId,
        displayName: importedBoard.displayName,
        startedAt,
        endedAt: new Date(Date.parse(startedAt) + 149_999_000).toISOString(),
        sampleCount: 150_000,
        eventCount: 2,
        ridingElapsedMs: 134_999_000,
        totalElapsedMs: 149_999_000,
        distanceM: 810_000,
        maxSpeedMps: 10,
        maxDutyCycle: 0.849,
      },
      ...this.sessionCatalog,
    ];
    this.change({ ...this.state, savedDevices: this.refreshSummaries(this.sessionCatalog) }, true);
    return sessionId;
  }

  private sessionList(
    deviceHistoryId: string,
    cursor: unknown,
    limit: number
  ):
    | { value: SessionListPage; error?: undefined }
    | { value?: undefined; error: { code: BackendError['code']; message: string } } {
    const ordered = this.sessionCatalog
      .filter((item) => item.deviceHistoryId === deviceHistoryId)
      .sort(compareSessions);
    let start = 0;
    if (cursor !== undefined && cursor !== null) {
      if (typeof cursor !== 'string' || Buffer.byteLength(cursor, 'utf8') > MAX_SESSION_LIST_CURSOR_BYTES) {
        return { error: { code: 'invalid_request', message: 'Session cursor is invalid' } };
      }
      let decoded: unknown;
      try {
        decoded = JSON.parse(Buffer.from(cursor, 'base64').toString('utf8'));
      } catch {
        return { error: { code: 'invalid_request', message: 'Session cursor is invalid' } };
      }
      const context = asObject(decoded);
      const lastStartedAt = context ? string(context.lastStartedAt) : null;
      const lastSessionId = context ? string(context.lastSessionId) : null;
      if (
        !context ||
        context.version !== 1 ||
        context.query !== 'startedAt.desc.sessionId.desc' ||
        context.deviceHistoryId !== deviceHistoryId ||
        !lastStartedAt ||
        !lastSessionId
      ) {
        return { error: { code: 'invalid_request', message: 'Session cursor is invalid' } };
      }
      if (context.catalogRevision !== this.state.sessionCatalogRevision) {
        return { error: { code: 'conflict', message: 'Session cursor is stale' } };
      }
      start = ordered.findIndex((item) => item.startedAt === lastStartedAt && item.sessionId === lastSessionId) + 1;
      if (start === 0) return { error: { code: 'conflict', message: 'Session cursor no longer matches the catalog' } };
    }
    const items = ordered.slice(start, start + limit);
    const last = items.at(-1);
    const nextCursor =
      start + items.length < ordered.length && last
        ? Buffer.from(
            JSON.stringify({
              version: 1,
              query: 'startedAt.desc.sessionId.desc',
              deviceHistoryId,
              catalogRevision: this.state.sessionCatalogRevision,
              lastStartedAt: last.startedAt,
              lastSessionId: last.sessionId,
            }),
            'utf8'
          ).toString('base64')
        : null;
    return { value: { items, catalogRevision: this.state.sessionCatalogRevision, nextCursor } };
  }

  private startOperation(
    kind: SessionOperationState['kind'],
    sessionId: string | null,
    scope: SessionOperationState['scope'],
    targetResolutionSeconds: number
  ): SessionOperationState {
    const operation: SessionOperationState = {
      operationId: `${kind}-${this.state.revision + 1}`,
      kind,
      status: kind === 'prunePreview' ? 'completed' : 'queued',
      sessionId,
      scope,
      targetResolutionSeconds,
      completedSessions: kind === 'prunePreview' ? (scope === 'one' ? 1 : this.sessionCatalog.length) : 0,
      eligibleSessions: scope === 'one' ? 1 : this.sessionCatalog.length,
      error: null,
    };
    this.operation = operation;
    this.change({ ...this.state, sessionOperation: operation });
    if (kind === 'prune') {
      this.operationTimer = setTimeout(() => {
        this.operationTimer = undefined;
        if (this.operation?.operationId !== operation.operationId || this.operation.status !== 'queued') return;
        const completed: SessionOperationState = {
          ...operation,
          status: 'completed',
          completedSessions: operation.eligibleSessions ?? 0,
        };
        this.operation = completed;
        this.change({
          ...this.state,
          sessionCatalogRevision: this.state.sessionCatalogRevision + 1,
          sessionOperation: completed,
        });
      }, 0);
      this.operationTimer.unref?.();
    }
    return operation;
  }

  private cancelOperation(): SessionOperationState {
    const operation = this.operation!;
    if (this.operationTimer) clearTimeout(this.operationTimer);
    this.operationTimer = undefined;
    const cancelled = { ...operation, status: 'cancelled' as const };
    this.operation = cancelled;
    this.change({ ...this.state, sessionOperation: cancelled });
    return cancelled;
  }

  private scheduleDiscoveryExpiry() {
    if (this.discoveryExpiryTimer) clearTimeout(this.discoveryExpiryTimer);
    this.discoveryExpiryTimer = setTimeout(() => {
      this.discoveryExpiryTimer = undefined;
      if (!this.state.discoveredDevices.length) return;
      this.change({ ...this.state, discoveredDevices: [] });
    }, 120_000);
    this.discoveryExpiryTimer.unref();
  }

  private change(next: AppState, catalogChanged = false) {
    const estimate = this.battery.update(
      next.activeConnection,
      this.profiles.get(next.activeConnection?.savedDeviceId ?? '')
    );
    if (next.activeConnection)
      next = {
        ...next,
        activeConnection: {
          ...next.activeConnection,
          batteryEstimate: estimate,
          liveWidgetSlots:
            next.activeConnection.trackingMode === 'saved'
              ? ((this.profiles.get(next.activeConnection.savedDeviceId ?? '')?.values.liveWidgetSlots as
                  | LiveWidgetSlots
                  | undefined) ?? [...defaultLiveWidgetSlots])
              : next.settings.liveWidgetSlots,
        },
      };
    this.state = {
      ...next,
      savedDevices: catalogChanged ? this.refreshSummaries(this.sessionCatalog, next.savedDevices) : next.savedDevices,
      revision: this.state.revision + 1,
      sessionCatalogRevision: catalogChanged
        ? this.state.sessionCatalogRevision + 1
        : this.state.sessionCatalogRevision,
    };
    this.publish({ event: 'state.changed', data: this.state });
  }

  private refreshSummaries(catalog: SessionSummary[], savedDevices = this.state.savedDevices) {
    return savedDevices.map((device) => {
      const sessions = catalog.filter((item) => item.deviceHistoryId === device.deviceHistoryId);
      const distances = sessions
        .map((item) => item.distanceM)
        .filter((value): value is number => typeof value === 'number');
      return {
        ...device,
        summary: {
          sessionCount: sessions.length,
          totalRidingElapsedMs: sessions.reduce((total, item) => total + item.ridingElapsedMs, 0),
          totalDistanceM: distances.length ? distances.reduce((total, value) => total + value, 0) : null,
          mostRecentSessionAt: sessions.slice().sort(compareSessions)[0]?.startedAt ?? null,
        },
      };
    });
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

function validRefloatIdentity(value: unknown): RefloatConfigurationIdentity | null {
  const identity = asObject(value);
  if (
    !identity ||
    identity.packageName !== 'refloat' ||
    typeof identity.packageVersion !== 'string' ||
    typeof identity.schemaFingerprint !== 'string' ||
    !/^[0-9a-f]{64}$/.test(identity.schemaFingerprint) ||
    integer(identity.wireSignature) === null ||
    Number(identity.wireSignature) < 0 ||
    Number(identity.wireSignature) > 0xffff_ffff
  )
    return null;
  return identity as unknown as RefloatConfigurationIdentity;
}

function sameRefloatIdentity(left: RefloatConfigurationIdentity, right: RefloatConfigurationIdentity): boolean {
  return (
    left.packageName === right.packageName &&
    left.packageVersion === right.packageVersion &&
    left.schemaFingerprint === right.schemaFingerprint &&
    left.wireSignature === right.wireSignature
  );
}

function validRefloatValues(value: Record<string, unknown>): boolean {
  const expected = refloatFields.map((field) => field.name).sort();
  if (Object.keys(value).sort().join('\0') !== expected.join('\0')) return false;
  if (Buffer.byteLength(JSON.stringify(value), 'utf8') > MAX_REFLOAT_CONFIGURATION_JSON_BYTES) return false;
  return refloatFields.every((field) => {
    const typed = asObject(value[field.name]);
    if (!typed || Object.keys(typed).sort().join(',') !== 'type,value') return false;
    switch (field.type) {
      case 'double':
        return (
          typed.type === 'double' &&
          typeof typed.value === 'number' &&
          Number.isFinite(typed.value) &&
          typed.value >= (field.minimum ?? -Infinity) &&
          typed.value <= (field.maximum ?? Infinity)
        );
      case 'integer':
      case 'enum':
      case 'bitfield':
        return typed.type === 'integer' && Number.isSafeInteger(typed.value);
      case 'boolean':
        return typed.type === 'boolean' && typeof typed.value === 'boolean';
      case 'string':
        return (
          typed.type === 'string' && typeof typed.value === 'string' && Buffer.byteLength(typed.value, 'utf8') <= 4_096
        );
    }
  });
}

function boundedText(value: unknown, maximumBytes: number, nullable: boolean): string | null | undefined {
  if (nullable && value === null) return null;
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  if (!trimmed || Buffer.byteLength(trimmed, 'utf8') > maximumBytes) return nullable ? undefined : null;
  return trimmed;
}

function encodeRefloatPageCursor(page: number): string {
  return Buffer.from(JSON.stringify({ page, fingerprint: refloatIdentity.schemaFingerprint }), 'utf8').toString(
    'base64'
  );
}

function decodeRefloatPageCursor(value: unknown): number | null {
  if (value === undefined || value === null) return 0;
  if (typeof value !== 'string' || Buffer.byteLength(value, 'utf8') > MAX_REFLOAT_CONFIGURATION_CURSOR_BYTES)
    return null;
  try {
    const decoded = JSON.parse(Buffer.from(value, 'base64').toString('utf8')) as Record<string, unknown>;
    return decoded.fingerprint === refloatIdentity.schemaFingerprint && Number.isInteger(decoded.page)
      ? Number(decoded.page)
      : null;
  } catch {
    return null;
  }
}

function encodeTuneCursor(offset: number): string {
  return Buffer.from(JSON.stringify({ offset }), 'utf8').toString('base64');
}

function decodeTuneCursor(value: unknown): number | null {
  if (value === undefined || value === null) return 0;
  if (typeof value !== 'string' || Buffer.byteLength(value, 'utf8') > MAX_REFLOAT_TUNE_CURSOR_BYTES) return null;
  try {
    const decoded = JSON.parse(Buffer.from(value, 'base64').toString('utf8')) as Record<string, unknown>;
    return Number.isInteger(decoded.offset) && Number(decoded.offset) >= 0 ? Number(decoded.offset) : null;
  } catch {
    return null;
  }
}

function isRefloatControl(value: unknown): value is RefloatControl {
  return value === 'package' || value === 'flywheel' || value === 'leds' || value === 'headlights';
}

function refloatControlField(
  control: RefloatControl
): 'packageEnabled' | 'flywheelEnabled' | 'ledsEnabled' | 'headlightsEnabled' {
  switch (control) {
    case 'package':
      return 'packageEnabled';
    case 'flywheel':
      return 'flywheelEnabled';
    case 'leds':
      return 'ledsEnabled';
    case 'headlights':
      return 'headlightsEnabled';
  }
}

function shutdownAvailable(active: ActiveConnection): boolean {
  return (
    active.state === 'ready' &&
    active.firmware?.accessMode === 'readWrite' &&
    ['6.05', '6.06', '7.00'].includes(active.firmware.vescVersion) &&
    active.latestSample?.rideState === 'idle' &&
    active.refloatControls?.flywheelEnabled !== true &&
    active.refloatControls?.pendingAction === null &&
    active.controllerActions.pendingAction === null
  );
}

function validBmsConnection(value: BmsState['connection']): boolean {
  return (
    value === null ||
    value.kind === 'direct' ||
    (value.kind === 'can' && Number.isInteger(value.canId) && value.canId >= 0 && value.canId <= 253)
  );
}

function compareBmsCandidates(left: BmsCandidate, right: BmsCandidate): number {
  const leftId = left.connection.kind === 'can' ? left.connection.canId : -1;
  const rightId = right.connection.kind === 'can' ? right.connection.canId : -1;
  return leftId - rightId;
}

function validBmsState(value: BmsState): boolean {
  if (!validBmsConnection(value.connection) || value.candidates.length > 16) return false;
  if (value.candidates.some((candidate) => !validBmsConnection(candidate.connection))) return false;
  const snapshot = value.latestSnapshot;
  if (snapshot) {
    if (
      snapshot.cellVoltagesV.length > 50 ||
      snapshot.balancing.length !== snapshot.cellVoltagesV.length ||
      snapshot.temperaturesC.length > 50 ||
      (snapshot.status !== null && new TextEncoder().encode(snapshot.status).length > 40) ||
      !Number.isInteger(snapshot.reportedCanId) ||
      snapshot.reportedCanId < 0 ||
      snapshot.reportedCanId > 253
    )
      return false;
    const numbers = Object.entries(snapshot)
      .filter(([key]) => !['observedAt', 'balancing', 'status', 'dataVersion'].includes(key))
      .flatMap(([, item]) => (Array.isArray(item) ? item : [item]));
    if (numbers.some((item) => typeof item !== 'number' || !Number.isFinite(item))) return false;
  }
  switch (value.status) {
    case 'not_found':
      return value.connection === null && value.candidates.length === 0 && snapshot === null;
    case 'attached':
      return value.connection !== null && value.candidates.length === 0 && snapshot !== null;
    case 'ambiguous':
      return value.connection === null && value.candidates.length >= 2 && snapshot === null;
    case 'discovering':
    case 'unavailable':
      return true;
  }
}

function compareSessions(left: SessionSummary, right: SessionSummary): number {
  const startedAt = right.startedAt.localeCompare(left.startedAt);
  return startedAt || right.sessionId.localeCompare(left.sessionId);
}

function emptySummary(): BoardHistorySummary {
  return {
    sessionCount: 0,
    totalRidingElapsedMs: 0,
    totalDistanceM: null,
    mostRecentSessionAt: null,
  };
}

function parseArchiveIdentity(archive: Uint8Array): { deviceHistoryId: string; displayName: string } {
  try {
    const text = new TextDecoder().decode(archive);
    const value = JSON.parse(text) as { deviceHistoryId?: unknown; displayName?: unknown };
    if (
      typeof value.deviceHistoryId !== 'string' ||
      !/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/.test(value.deviceHistoryId)
    ) {
      throw new Error('Session archive is missing a valid board identity');
    }
    return {
      deviceHistoryId: value.deviceHistoryId,
      displayName:
        typeof value.displayName === 'string' && value.displayName.trim()
          ? value.displayName.trim().slice(0, 128)
          : 'Imported board',
    };
  } catch {
    throw new Error('Session archive is missing a valid board identity');
  }
}
