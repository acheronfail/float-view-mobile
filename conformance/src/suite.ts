import assert from 'node:assert/strict';
import type {
  AppState,
  BackendMethod,
  ConnectionIssueCode,
  RefloatConfigurationField,
  RefloatConfigurationIdentity,
  RefloatConfigurationSchemaPage,
} from '../../frontend/src/lib/backend/contract.ts';
import { ConformanceClient, ConformanceError } from './client.ts';
import { exerciseTransport } from './transport.ts';

export interface Target {
  name: string;
  start(): Promise<{ url: string; origin: string }>;
  endpoint(): Promise<{ url: string; origin: string }>;
  background?(): Promise<void>;
  foreground?(): Promise<void>;
  recreateFrontend?(): Promise<void>;
  loseFixture?(): Promise<void>;
  stop(): Promise<void>;
}

export interface PhysicalTarget extends Target {
  platform: 'android' | 'ios' | 'macos';
  productionBluetooth: true;
  prepareSuite?(report: (message: string) => void): Promise<void>;
  /** Prepare OS-level location permissions without changing app settings. */
  prepareLocation?(report: (message: string) => void): Promise<void>;
  finishSuite?(report: (message: string) => void): Promise<void>;
  verifyFilePresentation?(): Promise<void>;
}

export async function runConformance(target: Target) {
  let client!: ConformanceClient;
  try {
    const endpoint = await target.start();
    client = await ConformanceClient.open(endpoint.url, endpoint.origin);
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
    const scanned = await client.waitForState((state) => state.discoveredDevices.length > 0, {
      afterRevision: initial.revision,
    });
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

    await exerciseProductMethods(
      client,
      restored,
      ['android', 'ios', 'macos', 'direct'].includes(target.name),
      target.recreateFrontend
    );
    if (['android', 'ios', 'macos', 'direct'].includes(target.name)) {
      await exerciseTransport(await target.endpoint());
    }
    if (target.loseFixture) {
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const observed = await client.waitForState((s) => s.discoveredDevices.length > 0, { afterRevision: 0 });
      const deviceId = observed.discoveredDevices[0]!.deviceId;
      await client.request('device.connect', { deviceId, trackingMode: 'saved' });
      const recording = await client.waitForState(
        (s) =>
          s.activeConnection?.state === 'ready' &&
          !!s.activeConnection.latestSample &&
          !!s.activeConnection.activeSession,
        { afterRevision: observed.revision }
      );
      const began = performance.now();
      await target.loseFixture();
      await client.waitForState((s) => s.activeConnection?.state !== 'ready', {
        afterRevision: recording.revision,
        timeoutMs: 5_000,
      });
      assert.ok(performance.now() - began < 5_000, 'fixture loss was not bounded');
    }
    return {
      target: target.name,
      revision: (await client.request('state.get', {})).revision,
    };
  } finally {
    client?.close();
    await target.stop();
  }
}

async function exerciseProductMethods(
  client: ConformanceClient,
  state: AppState,
  expectSoftwareBms: boolean,
  recreateFrontend?: () => Promise<void>
) {
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

  await createCompletedSession(client, device.deviceId, expectSoftwareBms, recreateFrontend);
  await createCompletedSession(client, device.deviceId, expectSoftwareBms);

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
  const originalMarkers = await client.sessionData({
    kind: 'markers',
    sessionId: session.sessionId,
    snapshot: detail.snapshot,
    range: historyRange,
    limit: 256,
  });
  assert.equal(detail.markerCount, 2);

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
  const prunedMarkers = await client.sessionData({
    kind: 'markers',
    sessionId: session.sessionId,
    snapshot: afterPrune.snapshot,
    range: historyRange,
    limit: 256,
  });
  assert.equal(afterPrune.markerCount, detail.markerCount);
  if (originalMarkers.kind === 'markers' && prunedMarkers.kind === 'markers')
    assert.deepEqual(prunedMarkers.markers, originalMarkers.markers);

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

async function createCompletedSession(
  client: ConformanceClient,
  deviceId: string,
  expectSoftwareBms: boolean,
  recreateFrontend?: () => Promise<void>
) {
  await client.request('bluetooth.scan', { timeoutMs: 100 });
  const before = await client.request('state.get', {});
  assert.ok(
    before.discoveredDevices.some((device) => device.deviceId === deviceId),
    'scan did not rediscover the session device'
  );
  await client.request('device.connect', { deviceId, trackingMode: 'saved' });
  let connected = await client.waitForState(
    (value) => value.activeConnection?.deviceId === deviceId && value.activeConnection.state === 'ready',
    { afterRevision: before.revision }
  );
  if (expectSoftwareBms && connected.activeConnection?.bms.status !== 'attached') {
    connected = await client.waitForState(
      (value) =>
        value.activeConnection?.deviceId === deviceId &&
        value.activeConnection.state === 'ready' &&
        value.activeConnection.bms.status === 'attached',
      { afterRevision: connected.revision }
    );
  }
  assertState(connected);
  assert.ok(connected.activeConnection);
  assert.equal(connected.activeConnection.deviceKind, 'board');
  const savedDeviceId = connected.activeConnection.savedDeviceId;
  assert.equal(connected.activeConnection.displayName, connected.activeConnection.name);
  assert.ok(connected.activeConnection.savedDeviceId);
  assert.ok(connected.activeConnection.activeSession);
  await exerciseBatteryEstimate(client, connected);
  if (recreateFrontend) {
    const beforeRecreation = await client.request('state.get', {});
    await recreateFrontend();
    const afterRecreation = await client.waitForState(
      (state) =>
        (state.activeConnection?.latestSample?.sequence ?? 0) >
        (beforeRecreation.activeConnection?.latestSample?.sequence ?? 0),
      { afterRevision: beforeRecreation.revision, timeoutMs: 10_000 }
    );
    assert.equal(
      afterRecreation.activeConnection?.activeSession?.sessionId,
      beforeRecreation.activeConnection?.activeSession?.sessionId,
      'frontend recreation replaced the recording session'
    );
    assert.equal(afterRecreation.activeConnection?.state, 'ready');
  }
  assert.ok(connected.activeConnection.controllerConnection);
  assert.ok(
    connected.activeConnection.controllerConnection.kind === 'direct' ||
      (connected.activeConnection.controllerConnection.kind === 'can' &&
        connected.activeConnection.controllerConnection.canId >= 0 &&
        connected.activeConnection.controllerConnection.canId <= 253)
  );
  assert.deepEqual(connected.activeConnection.controllerCandidates, []);
  assert.equal(connected.activeConnection.connectionIssue, null);
  if (expectSoftwareBms) {
    assert.deepEqual(connected.activeConnection.bms.connection, { kind: 'can', canId: 42 });
    assert.equal(connected.activeConnection.bms.status, 'attached');
    assert.equal(connected.activeConnection.bms.latestSnapshot?.reportedCanId, 42);
    assert.equal(connected.activeConnection.bms.latestSnapshot?.status, 'OK');
  }
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
  await exerciseRefloatConfiguration(client, connected);
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
  if (connected.activeConnection.refloatControls?.status === 'ready') {
    const controls = connected.activeConnection.refloatControls;
    if (controls.packageEnabled !== null && connected.activeConnection.latestSample?.rideState === 'idle') {
      const idempotent = await client.request('refloat.control.set', {
        control: 'package',
        enabled: controls.packageEnabled,
      });
      assert.deepEqual(idempotent, controls, 'idempotent control request changed authoritative state');
    }
    if (controls.fatalErrorClearable) {
      try {
        const cleared = await client.request('refloat.fatalError.clear', {});
        assert.equal(cleared.fatalErrorLatched, false);
        assert.equal(cleared.pendingAction, null);
      } catch (error) {
        if (!(error instanceof ConformanceError) || error.error.code !== 'invalid_state') throw error;
        const changed = await client.request('state.get', {});
        assert.ok(changed.revision > connected.revision, 'fatal clear failed without an intervening state change');
        assert.equal(changed.activeConnection?.latestSample?.rideState, 'riding');
        assert.equal(changed.activeConnection?.refloatControls?.fatalErrorClearable, false);
      }
    } else {
      await expectError(client.request('refloat.fatalError.clear', {}), 'invalid_state');
    }
  }
  await expectError(client.request('controller.shutdown', { force: true } as never), 'invalid_request');
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
  await expectError(client.request('controller.shutdown', {}), 'invalid_state');
  await expectError(client.request('device.connect', { deviceId, trackingMode: 'saved' }), 'invalid_state');
  const markerSession = required(ridingState.activeConnection?.activeSession, 'No saved marker session');
  const markerOperation = '00000000-0000-4000-8000-000000000001';
  await expectError(
    client.request('session.marker.add', { sessionId: 'missing-session', operationId: markerOperation }),
    'invalid_state'
  );
  const savedMarker = await client.request('session.marker.add', {
    sessionId: markerSession.sessionId,
    operationId: markerOperation,
  });
  const retryMarker = await client.request('session.marker.add', {
    sessionId: markerSession.sessionId,
    operationId: markerOperation,
  });
  assert.deepEqual(retryMarker.marker, savedMarker.marker, 'Marker retries must be idempotent');
  assert.ok(savedMarker.snapshot.committedEventSequence >= savedMarker.marker.sequence);
  await new Promise((resolve) => setTimeout(resolve, 550));
  const secondMarker = await client.request('session.marker.add', {
    sessionId: markerSession.sessionId,
    operationId: '00000000-0000-4000-8000-000000000002',
  });
  await client.request('device.disconnect', {});
  await client.waitForState((value) => value.activeConnection === null, {
    afterRevision: ridingState.revision,
  });
  const markerMetadata = await client.request('session.get', { sessionId: markerSession.sessionId });
  assert.equal(markerMetadata.markerCount, 2);
  const markerRequest = {
    kind: 'markers' as const,
    sessionId: markerSession.sessionId,
    snapshot: markerMetadata.snapshot,
    range: { startElapsedMs: 0, endElapsedMs: markerMetadata.summary.totalElapsedMs },
    limit: 1,
  };
  const markerPage = await client.sessionData(markerRequest);
  assert.equal(markerPage.kind, 'markers');
  if (markerPage.kind === 'markers') {
    assert.deepEqual(markerPage.markers, [savedMarker.marker]);
    assert.ok(markerPage.nextCursor);
    const next = await client.sessionData({ ...markerRequest, cursor: markerPage.nextCursor });
    assert.equal(next.kind, 'markers');
    if (next.kind === 'markers') {
      assert.deepEqual(next.markers, [secondMarker.marker]);
      assert.equal(next.nextCursor, null);
    }
    await assert.rejects(
      client.sessionData({ ...markerRequest, sessionId: 'foreign', cursor: markerPage.nextCursor }),
      /HTTP 400/
    );
    await assert.rejects(
      client.sessionData({
        ...markerRequest,
        range: { startElapsedMs: 1, endElapsedMs: markerMetadata.summary.totalElapsedMs },
        cursor: markerPage.nextCursor,
      }),
      /HTTP 400/
    );
  }
  await expectError(
    client.request('session.marker.add', { sessionId: markerSession.sessionId, operationId: markerOperation }),
    'invalid_state'
  );
  const completed = await client.request('state.get', {});
  assert.ok(savedDeviceId);
  const page = await client.request('session.list', { savedDeviceId, limit: 100 });
  assert.ok(
    page.items.some((item) => item.state === 'completed'),
    'disconnect did not retain the session'
  );
}

export async function exerciseRefloatConfiguration(
  client: ConformanceClient,
  connected: AppState,
  requireAvailable = false
) {
  const opened = await client.request('refloat.configuration.open', {});
  assert.ok(['loading', 'ready', 'unavailable'].includes(opened.availability));
  let ready = opened;
  if (opened.availability === 'unavailable') {
    if (!requireAvailable) return;
    const state = await client.waitForState(
      (value) => value.activeConnection?.refloatConfiguration?.availability !== 'unavailable',
      { afterRevision: connected.revision, timeoutMs: 15_000 }
    );
    ready = required(state.activeConnection?.refloatConfiguration, 'configuration summary disappeared');
  }
  if (ready.availability !== 'ready' || ready.pendingOperation !== null || !ready.currentLoaded) {
    const state = await client.waitForState(
      (value) => {
        const configuration = value.activeConnection?.refloatConfiguration;
        return (
          configuration?.availability === 'ready' &&
          configuration.pendingOperation === null &&
          configuration.currentLoaded
        );
      },
      { afterRevision: connected.revision, timeoutMs: requireAvailable ? 120_000 : 15_000 }
    );
    ready = required(state.activeConnection?.refloatConfiguration, 'configuration summary disappeared');
  }
  assert.equal(ready.pendingOperation, null);
  assert.equal(typeof ready.packageVersion, 'string');
  assert.equal(typeof ready.schemaFingerprint, 'string');
  assert.equal(typeof ready.wireSignature, 'number');

  const fields: RefloatConfigurationField[] = [];
  let cursor: string | null = null;
  let identity: RefloatConfigurationIdentity | undefined;
  do {
    const page: RefloatConfigurationSchemaPage = await client.request('refloat.configuration.schema', { cursor });
    identity ??= page.identity;
    assert.deepEqual(page.identity, identity);
    fields.push(...page.fields);
    cursor = page.nextCursor;
    assert.ok(fields.length <= 512, 'configuration schema exceeded field bound');
  } while (cursor);
  assert.ok(fields.length > 0);

  const current = await client.request('refloat.configuration.read', { source: 'current' });
  const defaults = await client.request('refloat.configuration.read', { source: 'default' });
  assert.deepEqual(current.identity, identity);
  assert.deepEqual(defaults.identity, identity);
  assert.equal(Object.keys(current.values).length, fields.length);
  assert.equal(Object.keys(defaults.values).length, fields.length);
  const currentKp = current.values.kp;
  assert.ok(
    !requireAvailable || currentKp?.type === 'double',
    'physical configuration fixture requires a numeric kp field'
  );
  const draftValues =
    currentKp?.type === 'double'
      ? {
          ...current.values,
          kp: { type: 'double' as const, value: currentKp.value + 1 },
        }
      : current.values;

  const tune = await client.request('refloat.tune.create', {
    name: 'Conformance configuration',
    description: 'Exact-version disposable conformance record',
    identity: current.identity,
    sourceRevision: current.revision,
    values: draftValues,
  });
  const listed = await client.request('refloat.tune.list', { identity: current.identity, limit: 1, cursor: null });
  assert.ok(listed.items.some((item) => item.tuneId === tune.tuneId));
  assert.deepEqual(await client.request('refloat.tune.get', { tuneId: tune.tuneId }), tune);
  const updated = await client.request('refloat.tune.update', {
    tuneId: tune.tuneId,
    expectedRevision: tune.revision,
    name: 'Conformance configuration updated',
    description: null,
    values: tune.values,
  });
  await expectError(
    client.request('refloat.tune.update', {
      tuneId: tune.tuneId,
      expectedRevision: tune.revision,
      name: tune.name,
      description: tune.description,
      values: tune.values,
    }),
    'conflict'
  );

  const idle = client.currentState()?.activeConnection?.latestSample?.rideState === 'idle';
  if (idle && ready.writable) {
    try {
      const written = await client.request('refloat.configuration.write', {
        packageVersion: current.identity.packageVersion,
        schemaFingerprint: current.identity.schemaFingerprint,
        wireSignature: current.identity.wireSignature,
        baseRevision: current.revision,
        values: draftValues,
      });
      assert.deepEqual(written.identity, current.identity);
      const applied = await client.request('refloat.tune.apply', {
        tuneId: updated.tuneId,
        expectedRevision: updated.revision,
        baseRevision: written.revision,
      });
      assert.deepEqual(applied.identity, current.identity);
    } finally {
      if (requireAvailable) {
        // Physical writes must leave the fixture's complete original configuration restored,
        // including when the write or subsequent Tune application fails.
        const latest = await client.request('refloat.configuration.read', { source: 'current' });
        await client.request('refloat.configuration.write', {
          packageVersion: current.identity.packageVersion,
          schemaFingerprint: current.identity.schemaFingerprint,
          wireSignature: current.identity.wireSignature,
          baseRevision: latest.revision,
          values: current.values,
        });
        const restored = await client.request('refloat.configuration.read', { source: 'current' });
        assert.deepEqual(restored.identity, current.identity);
        const expectedOriginal = { ...current.values };
        if (!current.identity.packageVersion.startsWith('1.0.') && 'meta.is_default' in expectedOriginal) {
          expectedOriginal['meta.is_default'] = { type: 'boolean', value: false };
        }
        assert.deepEqual(restored.values, expectedOriginal, 'physical configuration restore did not verify');
      }
    }
  } else {
    await expectError(
      client.request('refloat.tune.apply', {
        tuneId: updated.tuneId,
        expectedRevision: updated.revision,
        baseRevision: current.revision,
      }),
      'invalid_state'
    );
  }
  await client.request('refloat.tune.delete', { tuneId: updated.tuneId, expectedRevision: updated.revision });
  await expectError(client.request('refloat.tune.get', { tuneId: updated.tuneId }), 'not_found');
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
  assert.equal(state.batteryCellProfiles.length, 7);
  assert.equal(new Set(state.batteryCellProfiles.map((p) => p.id)).size, 7);
  assert.ok(state.batteryCellProfiles.every((p) => p.id.length > 0 && p.label.length > 0));
  assert.equal(state.batteryPresetCatalogVersion, 'packs-2026-09-30-v1');
  assert.equal(state.batteryPresets.length, 52);
  assert.equal(new Set(state.batteryPresets.map((p) => p.id)).size, 52);
  for (const p of state.batteryPresets) {
    assert.ok(p.id.length > 0 && p.id.length <= 64);
    assert.ok([p.label, p.manufacturer, p.cellLabel].every((v) => v.length > 0 && v.length <= 128));
    assert.ok(p.seriesCount >= 1 && p.seriesCount <= 36 && p.parallelCount >= 1 && p.parallelCount <= 20);
    assert.ok(state.batteryCellProfiles.some((c) => c.id === p.batteryCellProfileId));
    assert.ok(p.nominalCapacityAh === null || (p.nominalCapacityAh >= 0.1 && p.nominalCapacityAh <= 200));
  }
  const active = state.activeConnection;
  if (active) {
    const estimate = active.batteryEstimate;
    assert.equal(estimate.catalogVersion, 'floaty-2025-05-18-v1');
    assert.equal(estimate.sampleSequence, active.latestSample?.sequence ?? null);
    assert.ok(['load_compensated', 'load_clamped', 'voltage_only', 'unavailable'].includes(estimate.mode));
    if (estimate.level === null) {
      assert.equal(estimate.mode, 'unavailable');
      assert.ok(
        ['not_configured', 'series_mismatch', 'missing_voltage', 'not_applicable'].includes(estimate.unavailableReason!)
      );
    } else {
      assert.ok(Number.isFinite(estimate.level) && estimate.level >= 0 && estimate.level <= 1);
      assert.equal(estimate.unavailableReason, null);
    }
    if (active.trackingMode === 'guest' || active.deviceKind === 'bms') {
      assert.equal(estimate.level, null);
      assert.equal(estimate.unavailableReason, 'not_applicable');
    }
  }
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

  if (!active) return;
  assert.ok(active.deviceKind === null || active.deviceKind === 'board' || active.deviceKind === 'bms');
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

  assert.deepEqual(Object.keys(active.bms).sort(), ['candidates', 'connection', 'latestSnapshot', 'status']);
  assert.ok(['discovering', 'not_found', 'attached', 'unavailable', 'ambiguous'].includes(active.bms.status));
  assert.ok(active.bms.candidates.length <= 16);
  assert.ok(!JSON.stringify(active.bms).toLowerCase().includes('uuid'), 'BMS state exposed a private UUID');
  for (const candidate of active.bms.candidates) {
    assert.deepEqual(Object.keys(candidate).sort(), ['connection', 'hardwareName', 'vescVersion']);
    assertBmsConnection(candidate.connection);
    assert.ok((candidate.hardwareName?.length ?? 0) <= 80);
    assert.ok(candidate.vescVersion.length <= 32);
  }
  if (active.bms.connection) assertBmsConnection(active.bms.connection);
  if (active.bms.latestSnapshot) {
    const snapshot = active.bms.latestSnapshot;
    assert.ok(snapshot.cellVoltagesV.length <= 50);
    assert.equal(snapshot.balancing.length, snapshot.cellVoltagesV.length);
    assert.ok(snapshot.temperaturesC.length <= 50);
    assert.ok([...snapshot.cellVoltagesV, ...snapshot.temperaturesC].every(Number.isFinite));
    assert.ok(snapshot.balancing.every((value) => typeof value === 'boolean'));
    assert.ok(snapshot.status === null || new TextEncoder().encode(snapshot.status).length <= 40);
    assert.ok(snapshot.reportedCanId >= 0 && snapshot.reportedCanId <= 253);
  }

  if (active.refloatControls) {
    const controls = active.refloatControls;
    assert.deepEqual(Object.keys(controls).sort(), [
      'fatalErrorActive',
      'fatalErrorClearAvailable',
      'fatalErrorClearable',
      'fatalErrorLatched',
      'flywheelEnabled',
      'headlightsEnabled',
      'ledsEnabled',
      'lightingAvailable',
      'packageEnabled',
      'pendingAction',
      'status',
      'writable',
    ]);
    assert.ok(['loading', 'ready', 'unavailable'].includes(controls.status));
    assert.ok(
      controls.pendingAction === null ||
        ['package', 'flywheel', 'leds', 'headlights', 'clearFatalError'].includes(controls.pendingAction)
    );
    if (!controls.lightingAvailable) {
      assert.equal(controls.ledsEnabled, null);
      assert.equal(controls.headlightsEnabled, null);
    }
    if (!controls.fatalErrorClearAvailable) {
      assert.equal(controls.fatalErrorClearable, false);
    }
  }
  if (active.refloatConfiguration) {
    const configuration = active.refloatConfiguration;
    assert.deepEqual(Object.keys(configuration).sort(), [
      'availability',
      'currentLoaded',
      'defaultLoaded',
      'packageVersion',
      'pendingOperation',
      'schemaFingerprint',
      'wireSignature',
      'writable',
    ]);
    assert.ok(['loading', 'ready', 'unavailable'].includes(configuration.availability));
    assert.ok(
      configuration.pendingOperation === null ||
        ['schema', 'readCurrent', 'readDefault', 'write', 'applyTune'].includes(configuration.pendingOperation)
    );
    if (configuration.availability === 'ready') {
      assert.ok(configuration.packageVersion);
      assert.equal(configuration.schemaFingerprint?.length, 64);
      assert.ok(Number.isSafeInteger(configuration.wireSignature));
    }
  }

  assert.deepEqual(Object.keys(active.controllerActions).sort(), ['pendingAction', 'shutdownAvailable']);
  assert.ok(active.controllerActions.pendingAction === null || active.controllerActions.pendingAction === 'shutdown');
  if (active.controllerActions.pendingAction === 'shutdown') {
    assert.equal(active.controllerActions.shutdownAvailable, false);
  }

  if (active.connectionIssue) {
    assert.ok(stableConnectionIssueCodes.has(active.connectionIssue.code));
    assert.ok(active.connectionIssue.message.length > 0);
    assert.ok(active.connectionIssue.message.length <= 1_024);
  }
  if (active.state === 'ready') {
    assert.ok(active.controllerConnection, 'ready connection has no controller route');
    if (active.trackingMode === 'saved')
      assert.ok(active.activeSession, 'ready connection has no active durable session');
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

function assertBmsConnection(connection: { kind: string; canId?: number }) {
  if (connection.kind === 'direct') {
    assert.deepEqual(Object.keys(connection), ['kind']);
    return;
  }
  assert.equal(connection.kind, 'can');
  assert.deepEqual(Object.keys(connection).sort(), ['canId', 'kind']);
  assert.ok(Number.isInteger(connection.canId) && connection.canId! >= 0 && connection.canId! <= 253);
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
  'refloat.control.set',
  'refloat.fatalError.clear',
  'controller.shutdown',
  'refloat.configuration.open',
  'refloat.configuration.schema',
  'refloat.configuration.read',
  'refloat.configuration.write',
  'refloat.tune.list',
  'refloat.tune.get',
  'refloat.tune.create',
  'refloat.tune.update',
  'refloat.tune.delete',
  'refloat.tune.apply',
  'savedDevice.update',
  'savedDevice.delete',
  'settings.update',
  'boardProfile.get',
  'boardProfile.update',
  'session.list',
  'session.get',
  'session.marker.add',
  'session.export',
  'session.delete',
  'session.delete.cancel',
  'session.prune.preview',
  'session.prune.start',
  'session.prune.cancel',
];

async function exerciseBatteryEstimate(client: ConformanceClient, state: AppState) {
  const id = state.activeConnection!.savedDeviceId!;
  await exerciseBatteryPresets(client, id);
  const original = await client.request('boardProfile.get', { savedDeviceId: id });
  const series = state.activeConnection!.currentLimits?.batterySeriesCells ?? 18;
  const configured = await client.request('boardProfile.update', {
    savedDeviceId: id,
    expectedRevision: original.revision,
    patch: {
      batteryCellProfileId: 'generic-linear',
      batteryMinimumCellVoltageV: 3,
      seriesCount: series,
      parallelCount: 2,
    },
  });
  const estimated = await client.waitForState(
    (s) =>
      s.activeConnection?.batteryEstimate.profileRevision === configured.revision &&
      s.activeConnection.batteryEstimate.level != null,
    { afterRevision: state.revision }
  );
  assertState(estimated);
  const active = estimated.activeConnection!;
  const expected = Math.max(0, Math.min(1, (active.latestSample!.packVoltageV! / series - 3) / 1.2));
  assert.ok(Math.abs(active.batteryEstimate.level! - expected) < 1e-10);
  assert.equal(active.batteryEstimate.seriesCount, series);
  assert.equal(active.batteryEstimate.mode, 'voltage_only');
  for (const patch of [
    { batteryCellProfileId: 'unknown' },
    { batteryMinimumCellVoltageV: 4.2 },
    { batteryMinimumCellVoltageV: 2.69 },
  ]) {
    await expectError(
      client.request('boardProfile.update', { savedDeviceId: id, expectedRevision: configured.revision, patch }),
      'invalid_request'
    );
  }
  await expectError(
    client.request('boardProfile.update', {
      savedDeviceId: id,
      expectedRevision: original.revision,
      patch: { batteryCellProfileId: 'molicel-p45b' },
    }),
    'conflict'
  );
  assert.deepEqual(await client.request('boardProfile.get', { savedDeviceId: id }), configured);
  if (state.activeConnection!.currentLimits?.batterySeriesCells) {
    const mismatch = await client.request('boardProfile.update', {
      savedDeviceId: id,
      expectedRevision: configured.revision,
      patch: { seriesCount: series === 18 ? 20 : 18 },
    });
    const unavailable = await client.request('state.get', {});
    assert.equal(unavailable.activeConnection!.batteryEstimate.unavailableReason, 'series_mismatch');
    assert.equal(unavailable.activeConnection!.batteryEstimate.level, null);
    await client.request('boardProfile.update', {
      savedDeviceId: id,
      expectedRevision: mismatch.revision,
      patch: { seriesCount: series },
    });
  }
}

async function exerciseBatteryPresets(client: ConformanceClient, id: string) {
  let profile = await client.request('boardProfile.get', { savedDeviceId: id });
  const update = async (patch: Record<string, unknown>) => {
    profile = await client.request('boardProfile.update', {
      savedDeviceId: id,
      expectedRevision: profile.revision,
      patch,
    });
  };
  for (const [preset, curve, capacity] of [
    ['floatwheel-adv2-samsung-50s', 'samsung-50s', 10],
    ['fungineers-funwheel-x7s-molicel-p30b', 'generic-linear', 6],
  ] as const) {
    const revision = profile.revision;
    await update({ batteryPresetId: preset });
    assert.equal(profile.values.batteryPresetId, preset);
    assert.equal(profile.values.seriesCount, 20);
    assert.equal(profile.values.parallelCount, 2);
    assert.equal(profile.values.batteryCellProfileId, curve);
    assert.equal(profile.values.batteryCapacityAh, capacity);
    assert.equal(profile.values.batteryMinimumCellVoltageV, null);
    await expectError(
      client.request('boardProfile.update', {
        savedDeviceId: id,
        expectedRevision: revision,
        patch: { batteryPresetId: preset },
      }),
      'conflict'
    );
    for (const patch of [
      { batteryPresetId: preset, batteryCapacityAh: null },
      { batteryPresetId: 'unknown' },
      { batteryCapacityAh: 0.09 },
      { batteryCapacityAh: 201 },
      { batteryCapacityAh: '9' },
    ]) {
      await expectError(
        client.request('boardProfile.update', { savedDeviceId: id, expectedRevision: profile.revision, patch }),
        'invalid_request'
      );
    }
    assert.deepEqual(await client.request('boardProfile.get', { savedDeviceId: id }), profile);
    await update({ batteryCapacityAh: null });
    assert.equal(profile.values.batteryPresetId, preset);
    await update({ batteryPresetId: preset });
    assert.equal(profile.values.batteryCapacityAh, capacity);
    await update({ parallelCount: 3 });
    assert.equal(profile.values.batteryPresetId, null);
    assert.equal(profile.values.batteryCapacityAh, null);
  }
}
