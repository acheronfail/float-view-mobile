import {
  MAX_MESSAGE_BYTES,
  MAX_SESSION_LIST_CURSOR_BYTES,
  MAX_SESSION_LIST_PAGE,
  type AppState,
  type BackendError,
  type BackendMethod,
  type ConnectionIssueCode,
  type ControllerCandidate,
  type Request,
  type Response,
  type SessionMetadata,
  type SessionListPage,
  type SessionOperationState,
  type SessionPrunePreview,
  type SessionSummary,
  type StateChanged,
} from '../src/lib/backend/contract.ts';

const discoveredBoard = {
  deviceId: 'mock-board',
  name: 'Float View Test VESC',
  state: 'discovered' as const,
  lastSeenAt: '2026-01-01T00:00:00Z',
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
  return {
    summary,
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
    },
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
  private readonly publish: (event: StateChanged) => void;
  readonly profiles = new Map([
    [
      savedBoard.savedDeviceId,
      {
        savedDeviceId: savedBoard.savedDeviceId,
        revision: 0,
        values: {} as Record<string, unknown>,
        missingFields: ['seriesCount', 'parallelCount', 'motor'],
      },
    ],
  ]);
  private nextSession = 3;
  private sessionCatalog: SessionSummary[] = [secondSession, session];
  private activeSessionId?: string;
  private pendingCandidates: ControllerCandidate[] = [];
  private discoveryExpiryTimer?: ReturnType<typeof setTimeout>;
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
      historyGapThresholdSeconds: null,
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
      case 'state.get':
        return success(id, this.state);
      case 'bluetooth.requestPermission':
        return success(id, this.state.bluetooth);
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
        if (!deviceId) return failure(id, 'invalid_request', 'deviceId is required');
        const active = this.state.activeConnection;
        if (active && (active.deviceId !== deviceId || active.state !== 'failed')) {
          return failure(id, 'invalid_state', 'A board is already active');
        }
        if (!active && deviceId !== discoveredBoard.deviceId) {
          return failure(id, 'not_found', 'Device is no longer available');
        }
        this.activeSessionId = `session-${this.nextSession++}`;
        if (this.discoveryExpiryTimer) clearTimeout(this.discoveryExpiryTimer);
        this.discoveryExpiryTimer = undefined;
        const activeSession: SessionSummary = {
          ...session,
          sessionId: this.activeSessionId,
          startedAt: new Date().toISOString(),
          endedAt: null,
          state: 'active' as const,
          sampleCount: 0,
          ridingElapsedMs: 0,
          totalElapsedMs: 0,
        };
        this.sessionCatalog = [activeSession, ...this.sessionCatalog];
        this.change(
          {
            ...this.state,
            discoveredDevices: [],
            activeConnection: {
              ...discoveredBoard,
              state: 'ready',
              savedDeviceId: savedBoard.savedDeviceId,
              displayName: savedBoard.displayName,
              controllerConnection: { kind: 'direct' },
              controllerCandidates: [],
              connectionIssue: null,
              activeSession,
              latestSample: null,
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
                packageVersion: '1.2.3',
                hardwareName: 'Express',
                accessMode: 'readWrite',
                supported: true,
              },
            },
          },
          true
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
        const profile = this.profiles.get(active.savedDeviceId ?? '');
        if (!profile) return failure(id, 'invalid_state', 'Saved board is unavailable');
        const updated = {
          ...profile,
          revision: profile.revision + 1,
          values: { ...profile.values, preferredCanId: canId },
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
          displayName: customDisplayName ?? current.advertisedName ?? current.bluetoothIdentifier,
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
          'historyGapThresholdSeconds',
        ]);
        const historyGap = patch.historyGapThresholdSeconds;
        if (
          !Object.keys(patch).length ||
          Object.keys(patch).some((key) => !allowed.has(key)) ||
          ('themePreference' in patch && !['system', 'light', 'dark'].includes(String(patch.themePreference))) ||
          ('unitPreference' in patch && !['metric', 'imperial'].includes(String(patch.unitPreference))) ||
          ('historyGapThresholdSeconds' in patch &&
            historyGap !== null &&
            (typeof historyGap !== 'number' || !Number.isFinite(historyGap) || historyGap < 0.1 || historyGap > 3_600))
        ) {
          return failure(id, 'invalid_request', 'Settings patch is invalid');
        }
        const settings = {
          ...this.state.settings,
          ...patch,
          revision: this.state.settings.revision + 1,
        };
        this.change({ ...this.state, settings });
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
        const updated = {
          ...current,
          revision: current.revision + 1,
          values: { ...current.values, ...patch },
          missingFields: current.missingFields.filter((field) => !(field in patch)),
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
      case 'session.get': {
        const found = this.findSession(params);
        return found ? success(id, sessionMetadata(found)) : failure(id, 'not_found', 'Session was not found');
      }
      case 'session.list': {
        const limit = integer(params.limit);
        if (limit === null || limit < 1 || limit > MAX_SESSION_LIST_PAGE) {
          return failure(id, 'invalid_request', 'Session page size must be 1 through 100');
        }
        const page = this.sessionList(params.cursor, limit);
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
    if (!active || active.state !== 'ready' || !active.activeSession) return;
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
    const activeSession = {
      ...active.activeSession,
      sampleCount: sequence,
      ridingElapsedMs: elapsedMs,
      totalElapsedMs: elapsedMs,
      maxSpeedMps: latestSample.speedKph / 3.6,
      maxDutyCycle: latestSample.dutyCycle,
    };
    this.change({
      ...this.state,
      activeConnection: { ...active, latestSample, activeSession },
    });
  }

  get telemetryCadenceMs(): number {
    return this.state.settings.ridingPersistIntervalMs;
  }

  simulateConnectionState(state: 'ready' | 'reconnecting'): void {
    if (!this.state.activeConnection) return;
    this.change({
      ...this.state,
      activeConnection: { ...this.state.activeConnection, state },
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
          savedDeviceId: savedBoard.savedDeviceId,
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

  private sessionList(
    cursor: unknown,
    limit: number
  ):
    | { value: SessionListPage; error?: undefined }
    | { value?: undefined; error: { code: BackendError['code']; message: string } } {
    const ordered = [...this.sessionCatalog].sort(compareSessions);
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
        this.change({ ...this.state, sessionOperation: completed });
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
    this.state = {
      ...next,
      revision: this.state.revision + 1,
      sessionCatalogRevision: catalogChanged
        ? this.state.sessionCatalogRevision + 1
        : this.state.sessionCatalogRevision,
    };
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

function compareSessions(left: SessionSummary, right: SessionSummary): number {
  const startedAt = right.startedAt.localeCompare(left.startedAt);
  return startedAt || right.sessionId.localeCompare(left.sessionId);
}
