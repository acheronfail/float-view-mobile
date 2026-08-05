import assert from 'node:assert/strict';
import type { AppState, BackendMethod, ConnectionIssueCode } from '../../frontend/src/lib/backend/contract.ts';
import { ConformanceClient, ConformanceError } from './client.ts';

export interface Target {
  name: string;
  start(): Promise<{ url: string; origin: string }>;
  endpoint(): Promise<{ url: string; origin: string }>;
  background?(): Promise<void>;
  foreground?(): Promise<void>;
  stop(): Promise<void>;
}

export interface PhysicalTarget extends Target {
  platform: 'android' | 'ios' | 'macos';
  productionBluetooth: true;
  prepareSuite?(report: (message: string) => void): Promise<void>;
  /** Prepare OS-level location permissions without changing app settings. */
  prepareLocation?(report: (message: string) => void): Promise<void>;
  finishSuite?(report: (message: string) => void): Promise<void>;
}

export async function runConformance(target: Target) {
  const endpoint = await target.start();
  let client = await ConformanceClient.open(endpoint.url, endpoint.origin);
  try {
    const initial = await client.request('state.get', {});
    assertState(initial);
    await expectError(
      (
        client as unknown as {
          request(method: string, params: unknown): Promise<unknown>;
        }
      ).request('unknown.method', {}),
      'invalid_request'
    );

    const permission = await client.request('bluetooth.requestPermission', {});
    assert.equal(typeof permission.radio, 'string');
    await client.request('bluetooth.scan', { timeoutMs: 100 });
    const scanned = await client.request('state.get', {});
    assertState(scanned);
    assert.ok(scanned.revision >= initial.revision);

    client.close();
    const replacementEndpoint = await target.endpoint();
    client = await ConformanceClient.open(replacementEndpoint.url, replacementEndpoint.origin);
    const restored = await client.request('state.get', {});
    assertState(restored);
    assert.ok(restored.revision >= scanned.revision);
    assert.deepEqual(
      restored.discoveredDevices.map((device) => device.deviceId),
      scanned.discoveredDevices.map((device) => device.deviceId),
      'frontend recreation lost backend-owned state'
    );

    await exerciseProductMethods(client, restored);
    return {
      target: target.name,
      revision: (await client.request('state.get', {})).revision,
    };
  } finally {
    client.close();
    await target.stop();
  }
}

async function exerciseProductMethods(client: ConformanceClient, state: AppState) {
  const device = required(state.discoveredDevices[0], 'scan did not create a discovered device');
  const settings = await client.request('settings.update', {
    expectedRevision: state.settings.revision,
    patch: {
      minimumSessionDistance: 100,
      themePreference: 'dark',
      unitPreference: 'imperial',
    },
  });
  await expectError(
    client.request('settings.update', {
      expectedRevision: state.settings.revision,
      patch: { minimumSessionDistance: 101 },
    }),
    'conflict'
  );
  assert.equal(settings.minimumSessionDistance, 100);
  assert.equal(settings.themePreference, 'dark');
  assert.equal(settings.unitPreference, 'imperial');
  const locationSettings = await client.request('settings.update', {
    expectedRevision: settings.revision,
    patch: { locationRecordingEnabled: true, includeGpsLocationWhenExporting: true },
  });
  assert.equal(locationSettings.locationRecordingEnabled, true);
  assert.equal(locationSettings.includeGpsLocationWhenExporting, true);
  const locationState = await client.request('location.requestPermission', {});
  assert.ok(
    ['notDetermined', 'foregroundOnly', 'authorized', 'denied', 'restricted', 'unavailable'].includes(
      locationState.status
    )
  );
  await expectError(
    (
      client as unknown as {
        request(method: string, params: unknown): Promise<unknown>;
      }
    ).request('settings.update', {
      expectedRevision: locationSettings.revision,
      patch: { automaticSessionLinkingEnabled: true },
    }),
    'invalid_request'
  );

  // Software-host sessions are intentionally brief; retain them so the rest of
  // the canonical session surface can be exercised deterministically.
  await client.request('settings.update', {
    expectedRevision: locationSettings.revision,
    patch: { minimumSessionDistance: 0 },
  });

  await createCompletedSession(client, device.deviceId);
  await createCompletedSession(client, device.deviceId);

  const prepared = await client.request('state.get', {});
  const profileDevice = required(
    prepared.savedDevices.find((saved) => saved.advertisedName === device.name) ?? prepared.savedDevices[0],
    'connection did not create a saved device'
  );
  const profile = await client.request('boardProfile.get', {
    savedDeviceId: profileDevice.savedDeviceId,
  });
  assert.equal(typeof profileDevice.bluetoothIdentifier, 'string');
  assert.ok(profileDevice.bluetoothIdentifier);
  assert.ok(profileDevice.bluetoothIdentifier.length > 0);
  const renamed = await client.request('savedDevice.update', {
    savedDeviceId: profileDevice.savedDeviceId,
    expectedRevision: profileDevice.revision,
    customDisplayName: 'Conformance Board',
  });
  assert.equal(renamed.displayName, 'Conformance Board');
  assert.equal(renamed.customDisplayName, 'Conformance Board');
  await expectError(
    client.request('savedDevice.update', {
      savedDeviceId: profileDevice.savedDeviceId,
      expectedRevision: profileDevice.revision,
      customDisplayName: null,
    }),
    'conflict'
  );
  const resetName = await client.request('savedDevice.update', {
    savedDeviceId: profileDevice.savedDeviceId,
    expectedRevision: renamed.revision,
    customDisplayName: null,
  });
  assert.equal(resetName.displayName, resetName.advertisedName ?? resetName.bluetoothIdentifier);
  const updatedProfile = await client.request('boardProfile.update', {
    savedDeviceId: profile.savedDeviceId,
    expectedRevision: profile.revision,
    patch: { seriesCount: 18, parallelCount: 2, motor: 'hypercore_xr' },
  });
  assert.equal(updatedProfile.missingFields.length, 0);
  const preferredProfile = await client.request('boardProfile.update', {
    savedDeviceId: profile.savedDeviceId,
    expectedRevision: updatedProfile.revision,
    patch: { preferredCanId: 42 },
  });
  assert.equal(preferredProfile.values.preferredCanId, 42);
  assert.equal(preferredProfile.missingFields.length, 0);
  await expectError(
    client.request('boardProfile.update', {
      savedDeviceId: profile.savedDeviceId,
      expectedRevision: preferredProfile.revision,
      patch: { preferredCanId: 254 },
    }),
    'invalid_request'
  );
  const clearedPreference = await client.request('boardProfile.update', {
    savedDeviceId: profile.savedDeviceId,
    expectedRevision: preferredProfile.revision,
    patch: { preferredCanId: null },
  });
  assert.equal(clearedPreference.values.preferredCanId, null);
  await expectError(
    client.request('boardProfile.update', {
      savedDeviceId: profile.savedDeviceId,
      expectedRevision: profile.revision,
      patch: {},
    }),
    'conflict'
  );
  await expectError(
    client.request('boardProfile.get', {
      savedDeviceId: 'missing-saved-device',
    }),
    'not_found'
  );

  const current = await client.request('state.get', {});
  const firstPage = await client.request('session.list', { savedDeviceId: profileDevice.savedDeviceId, limit: 1 });
  const session = required(firstPage.items[0], 'connections did not create a session');
  assert.ok(firstPage.items.length <= 1);
  if (firstPage.nextCursor) {
    const secondPage = await client.request('session.list', {
      savedDeviceId: profileDevice.savedDeviceId,
      limit: 1,
      cursor: firstPage.nextCursor,
    });
    assert.ok(secondPage.items.length <= 1);
    assert.notEqual(secondPage.items[0]?.sessionId, firstPage.items[0]?.sessionId);
  }
  const detail = await client.request('session.get', {
    sessionId: session.sessionId,
  });
  assert.equal(detail.summary.sessionId, session.sessionId);
  assert.ok(detail.fields.length > 0 && detail.fields.length <= 64);
  assert.ok(Number.isSafeInteger(detail.snapshot.historyRevision));
  assert.ok(detail.storedSampleCount <= detail.recordedSampleCount);
  assert.doesNotMatch(JSON.stringify(detail), /controllerUuid|controllerUUID|launchToken|bluetoothIdentifier/);
  const historyRange = { startElapsedMs: 0, endElapsedMs: detail.snapshot.committedElapsedMs };
  const field = detail.fields[0]?.name;
  assert.ok(field, 'session metadata did not publish a numeric field');
  const overview = await client.sessionData({
    kind: 'series',
    sessionId: session.sessionId,
    snapshot: detail.snapshot,
    range: historyRange,
    fields: [field],
    buckets: 16,
    gapPolicy: { thresholdSeconds: null },
  });
  assert.equal(overview.kind, 'series');
  const samples = await client.sessionData({
    kind: 'samples',
    sessionId: session.sessionId,
    snapshot: detail.snapshot,
    range: historyRange,
    fields: [field],
    mode: 'window',
    limit: 100,
  });
  assert.equal(samples.kind, 'samples');
  if (samples.nextCursor) {
    const continuation = await client.sessionData({
      kind: 'samples',
      sessionId: session.sessionId,
      snapshot: detail.snapshot,
      range: historyRange,
      fields: [field],
      mode: 'window',
      limit: 100,
      cursor: samples.nextCursor,
    });
    assert.equal(continuation.kind, 'samples');
    const sampleElapsed = samples.elapsedMs.at(-1) ?? -1;
    const sampleSequence = samples.sequence.at(-1) ?? -1;
    assert.ok(
      continuation.elapsedMs.every(
        (elapsed, index) =>
          elapsed > sampleElapsed || (elapsed === sampleElapsed && continuation.sequence[index]! > sampleSequence)
      )
    );
  }
  const events = await client.sessionData({
    kind: 'events',
    sessionId: session.sessionId,
    snapshot: detail.snapshot,
    range: historyRange,
    limit: 100,
  });
  assert.equal(events.kind, 'events');
  if (events.nextCursor) {
    const eventContinuation = await client.sessionData({
      kind: 'events',
      sessionId: session.sessionId,
      snapshot: detail.snapshot,
      range: historyRange,
      limit: 100,
      cursor: events.nextCursor,
    });
    assert.equal(eventContinuation.kind, 'events');
    const eventElapsed = events.page.elapsedMs.at(-1) ?? -1;
    const eventSequence = events.page.sequence.at(-1) ?? -1;
    assert.ok(
      eventContinuation.page.elapsedMs.every(
        (elapsed, index) =>
          elapsed > eventElapsed ||
          (elapsed === eventElapsed && eventContinuation.page.sequence[index]! > eventSequence)
      )
    );
  }
  await expectSessionDataFailure(
    client,
    {
      kind: 'samples',
      sessionId: session.sessionId,
      snapshot: { ...detail.snapshot, historyRevision: detail.snapshot.historyRevision + 1 },
      range: historyRange,
      fields: [field],
      mode: 'window',
      limit: 1,
    },
    'stale revision'
  );
  const prunePreview = await client.request('session.prune.preview', {
    scope: 'one',
    sessionId: session.sessionId,
    targetResolutionSeconds: 5,
  });
  assert.equal(prunePreview.operation.kind, 'prunePreview');
  assert.equal(prunePreview.operation.status, 'completed');
  assert.ok(prunePreview.estimatedStoredSampleCount <= prunePreview.recordedSampleCount);
  const prune = await client.request('session.prune.start', {
    scope: 'one',
    sessionId: session.sessionId,
    targetResolutionSeconds: 5,
  });
  assert.equal(prune.kind, 'prune');
  assert.ok(['queued', 'running', 'completed'].includes(prune.status));
  if (prune.status !== 'completed') {
    const observed = await client.request('state.get', {});
    if (
      !observed.sessionOperation ||
      observed.sessionOperation.operationId !== prune.operationId ||
      !['completed', 'cancelled', 'failed'].includes(observed.sessionOperation.status)
    ) {
      const terminal = await client.waitForState(
        (value) => {
          const operation = value.sessionOperation;
          return (
            operation?.operationId === prune.operationId &&
            ['completed', 'cancelled', 'failed'].includes(operation.status)
          );
        },
        { afterRevision: observed.revision }
      );
      assert.ok(terminal.sessionOperation);
    }
  }
  assert.ok(Number.isSafeInteger(prune.completedSessions));
  const afterPrune = await client.request('session.get', { sessionId: session.sessionId });
  assert.ok(afterPrune.snapshot.historyRevision >= detail.snapshot.historyRevision);
  const cancelledPrune = await client.request('session.prune.cancel', { operationId: prune.operationId });
  assert.ok(['completed', 'cancelled'].includes(cancelledPrune.status));
  await client.request('session.export', {
    sessionId: session.sessionId,
    format: 'csv',
  });
  await expectError(client.request('session.get', { sessionId: 'missing-session' }), 'not_found');

  await client.request('session.delete', { sessionId: session.sessionId });
  const deleted = await client.request('session.list', { savedDeviceId: profileDevice.savedDeviceId, limit: 100 });
  assert.ok(!deleted.items.some((item) => item.sessionId === session.sessionId));
}

async function expectSessionDataFailure(
  client: ConformanceClient,
  request: Parameters<ConformanceClient['sessionData']>[0],
  label: string
) {
  await assert.rejects(client.sessionData(request), () => true, label);
}

async function createCompletedSession(client: ConformanceClient, deviceId: string) {
  await client.request('bluetooth.scan', { timeoutMs: 100 });
  const before = await client.request('state.get', {});
  assert.ok(
    before.discoveredDevices.some((device) => device.deviceId === deviceId),
    'scan did not rediscover the session device'
  );
  await client.request('device.connect', { deviceId, trackingMode: 'saved' });
  const connected = await client.waitForState(
    (value) => value.activeConnection?.deviceId === deviceId && value.activeConnection.state === 'ready',
    { afterRevision: before.revision }
  );
  assertState(connected);
  assert.ok(connected.activeConnection);
  const savedDeviceId = connected.activeConnection.savedDeviceId;
  assert.equal(connected.activeConnection.displayName, connected.activeConnection.name);
  assert.ok(connected.activeConnection.savedDeviceId);
  assert.ok(connected.activeConnection.activeSession);
  assert.ok(connected.activeConnection.controllerConnection);
  assert.ok(
    connected.activeConnection.controllerConnection.kind === 'direct' ||
      (connected.activeConnection.controllerConnection.kind === 'can' &&
        connected.activeConnection.controllerConnection.canId >= 0 &&
        connected.activeConnection.controllerConnection.canId <= 253)
  );
  assert.deepEqual(connected.activeConnection.controllerCandidates, []);
  assert.equal(connected.activeConnection.connectionIssue, null);
  if (connected.activeConnection.currentLimits) {
    const limits = connected.activeConnection.currentLimits;
    assert.deepEqual(Object.keys(limits).sort(), [
      'batteryCurrentMaxA',
      'batteryCurrentMinA',
      'batterySeriesCells',
      'motorCurrentMaxA',
      'motorCurrentMinA',
    ]);
    for (const value of Object.values(limits).filter((value) => value != null)) {
      assert.ok(Number.isFinite(value));
      assert.ok(Math.abs(value) > 0);
    }
  }
  if (connected.activeConnection.latestSample) {
    assert.deepEqual(Object.keys(connected.activeConnection.latestSample).sort(), [
      'alertReason',
      'ampHoursChargedAh',
      'ampHoursDrawnAh',
      'atrAccelerationDifferenceErpmPerSecond',
      'atrSetpointDeg',
      'atrSpeedBoost',
      'balancePitchDeg',
      'batteryCurrentA',
      'batteryLevel',
      'boosterCurrentA',
      'brakeTiltSetpointDeg',
      'charging',
      'chargingCurrentA',
      'chargingVoltageV',
      'controllerTempC',
      'darkride',
      'directionalMotorCurrentA',
      'distanceM',
      'dutyCycle',
      'elapsedMs',
      'erpm',
      'extensions',
      'fieldWeakeningCurrentA',
      'filteredMotorCurrentA',
      'firmwareFault',
      'footpadAdc1V',
      'footpadAdc2V',
      'footpadState',
      'motorCurrentA',
      'motorTempC',
      'observedAt',
      'odometerM',
      'packVoltageV',
      'packageMode',
      'packageState',
      'pitchDeg',
      'remoteInput',
      'remoteSetpointDeg',
      'requestedBalanceCurrentA',
      'rideState',
      'rollDeg',
      'sequence',
      'setpointAdjustment',
      'setpointDeg',
      'slowObservedAt',
      'speedKph',
      'stopCondition',
      'torqueTiltSetpointDeg',
      'turnTiltSetpointDeg',
      'wattHoursChargedWh',
      'wattHoursDrawnWh',
      'wheelslip',
    ]);
  }
  let ridingState: AppState;
  if (connected.activeConnection.latestSample?.rideState === 'riding') {
    ridingState = connected;
  } else {
    ridingState = await client.waitForState(
      (value) =>
        value.activeConnection?.deviceId === deviceId && value.activeConnection.latestSample?.rideState === 'riding',
      { afterRevision: connected.revision }
    );
  }
  await expectError(client.request('device.connect', { deviceId, trackingMode: 'saved' }), 'invalid_state');
  await client.request('device.disconnect', {});
  await client.waitForState((value) => value.activeConnection === null, {
    afterRevision: ridingState.revision,
  });
  const completed = await client.request('state.get', {});
  assert.ok(savedDeviceId);
  const page = await client.request('session.list', { savedDeviceId, limit: 100 });
  assert.ok(
    page.items.some((item) => item.state === 'completed'),
    'disconnect did not retain the session'
  );
}

const stableConnectionIssueCodes = new Set<ConnectionIssueCode>([
  'transport_setup_failed',
  'identification_timeout',
  'no_controller_found',
  'too_many_can_devices',
  'too_many_controllers',
  'preferred_controller_unavailable',
  'controller_changed',
  'unsupported_controller',
  'unsupported_refloat',
]);

export function assertState(state: AppState) {
  assert.ok(Number.isSafeInteger(state.revision));
  assert.ok(Array.isArray(state.discoveredDevices));
  assert.ok(Array.isArray(state.savedDevices));
  assert.ok(Number.isSafeInteger(state.sessionCatalogRevision));
  assert.ok(state.discoveredDevices.length <= 50);
  assert.ok(state.savedDevices.length <= 50);
  assert.equal(
    new Set(state.discoveredDevices.map((device) => device.deviceId)).size,
    state.discoveredDevices.length,
    'discovered devices contain duplicate IDs'
  );
  for (const device of state.discoveredDevices) {
    assert.ok(
      device.rssi === null || Number.isSafeInteger(device.rssi),
      'discovered device RSSI is not an integer or null'
    );
    assert.ok(device.savedDeviceId === null || typeof device.savedDeviceId === 'string');
  }
  for (const board of state.savedDevices) {
    assert.ok(board.bluetoothIdentifier === null || typeof board.bluetoothIdentifier === 'string');
    assert.deepEqual(Object.keys(board.summary).sort(), [
      'mostRecentSessionAt',
      'sessionCount',
      'totalDistanceM',
      'totalRidingElapsedMs',
    ]);
    assert.ok(Number.isSafeInteger(board.summary.sessionCount));
    assert.ok(Number.isSafeInteger(board.summary.totalRidingElapsedMs));
    assert.ok(board.summary.totalDistanceM === null || Number.isFinite(board.summary.totalDistanceM));
  }
  assert.deepEqual(Object.keys(state.settings).sort(), [
    'historyGapThresholdSeconds',
    'idlePersistIntervalMs',
    'idleProbeIntervalMs',
    'includeGpsLocationWhenExporting',
    'locationRecordingEnabled',
    'minimumSessionDistance',
    'postRideTailMs',
    'reconnectDurationMs',
    'revision',
    'ridingPersistIntervalMs',
    'themePreference',
    'unitPreference',
  ]);
  assert.ok(
    ['disabled', 'notDetermined', 'foregroundOnly', 'authorized', 'denied', 'restricted', 'unavailable'].includes(
      state.location.status
    )
  );
  assert.ok(['available', 'foregroundOnly', 'unavailable'].includes(state.location.backgroundCapture));
  assert.ok(['system', 'light', 'dark'].includes(state.settings.themePreference));
  assert.ok(['metric', 'imperial'].includes(state.settings.unitPreference));

  const active = state.activeConnection;
  if (!active) return;
  assert.ok(active.trackingMode === 'saved' || active.trackingMode === 'guest');
  if (active.trackingMode === 'guest') {
    assert.equal(active.savedDeviceId, null);
    assert.equal(active.activeSession, null);
    assert.equal(active.latestLocation, null);
  }
  const latestLocation = active.latestLocation;
  if (latestLocation) {
    assert.deepEqual(Object.keys(latestLocation).sort(), ['accuracyM', 'latitude', 'longitude', 'observedAt']);
    assert.ok(Number.isFinite(latestLocation.latitude));
    assert.ok(latestLocation.latitude >= -90 && latestLocation.latitude <= 90);
    assert.ok(Number.isFinite(latestLocation.longitude));
    assert.ok(latestLocation.longitude >= -180 && latestLocation.longitude <= 180);
    assert.ok(
      latestLocation.accuracyM === null || (Number.isFinite(latestLocation.accuracyM) && latestLocation.accuracyM >= 0)
    );
    assert.ok(typeof latestLocation.observedAt === 'string' && Number.isFinite(Date.parse(latestLocation.observedAt)));
  }
  assert.ok(active.controllerCandidates.length <= 16);
  assert.deepEqual(
    active.controllerCandidates.map((candidate) => candidate.canId),
    active.controllerCandidates.map((candidate) => candidate.canId).sort((left, right) => left - right),
    'controller candidates are not sorted by CAN ID'
  );
  assert.equal(
    new Set(active.controllerCandidates.map((candidate) => candidate.canId)).size,
    active.controllerCandidates.length,
    'controller candidates contain duplicate CAN IDs'
  );
  for (const candidate of active.controllerCandidates) {
    assert.ok(Number.isInteger(candidate.canId) && candidate.canId >= 0 && candidate.canId <= 253);
    assert.deepEqual(Object.keys(candidate).sort(), [
      'canId',
      'firmwareName',
      'hardwareName',
      'vescSupported',
      'vescVersion',
    ]);
    assert.ok((candidate.hardwareName?.length ?? 0) <= 80);
    assert.ok((candidate.firmwareName?.length ?? 0) <= 80);
  }
  assert.ok(
    !JSON.stringify(active.controllerCandidates).toLowerCase().includes('uuid'),
    'controller candidates exposed a private UUID'
  );

  if (active.connectionIssue) {
    assert.ok(stableConnectionIssueCodes.has(active.connectionIssue.code));
    assert.ok(active.connectionIssue.message.length > 0);
    assert.ok(active.connectionIssue.message.length <= 1_024);
  }
  if (active.state === 'ready') {
    assert.ok(active.controllerConnection, 'ready connection has no controller route');
    if (active.trackingMode === 'saved') assert.ok(active.activeSession, 'ready connection has no active durable session');
    else assert.equal(active.activeSession, null);
    assert.ok(active.firmware?.supported, 'ready connection has unsupported or absent firmware');
    assert.deepEqual(active.controllerCandidates, []);
    assert.equal(active.connectionIssue, null);
  } else if (active.state === 'selecting_controller') {
    assert.equal(active.controllerConnection, null);
    assert.ok(active.savedDeviceId);
    assert.ok(active.controllerCandidates.length >= 2);
    assert.equal(active.activeSession, null);
    assert.equal(active.connectionIssue, null);
  } else if (active.state === 'failed') {
    assert.ok(active.connectionIssue, 'failed connection has no stable issue');
    assert.deepEqual(active.controllerCandidates, []);
    assert.equal(active.activeSession, null);
  } else {
    assert.equal(active.connectionIssue, null);
    assert.deepEqual(active.controllerCandidates, []);
  }
}

async function expectError(promise: Promise<unknown>, code: string) {
  await assert.rejects(promise, (error: unknown) => {
    return error instanceof ConformanceError && error.error.code === code;
  });
}

function required<T>(value: T | null | undefined, message: string): T {
  assert.ok(value, message);
  return value;
}

export const mandatoryMethods: BackendMethod[] = [
  'state.get',
  'bluetooth.requestPermission',
  'location.requestPermission',
  'location.openSettings',
  'bluetooth.scan',
  'device.connect',
  'device.selectController',
  'device.disconnect',
  'savedDevice.update',
  'savedDevice.delete',
  'settings.update',
  'boardProfile.get',
  'boardProfile.update',
  'session.list',
  'session.get',
  'session.export',
  'session.delete',
  'session.delete.cancel',
  'session.prune.preview',
  'session.prune.start',
  'session.prune.cancel',
];
