import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { FixtureRunner } from '../fixture/runner.ts';
import {
  fixtureScenarios,
  physicalScenario,
  physicalScenarioMatrix,
  type PhysicalScenarioName,
} from '../fixture/scenarios.ts';
import { FixtureClient } from '../fixture/serial.ts';
import { ConformanceClient } from './client.ts';
import type { AppState, LocationState } from '../../frontend/src/lib/backend/contract.ts';
import { exerciseRefloatConfiguration, type PhysicalTarget } from './suite.ts';

type Progress = (message: string) => void;
type LocationCheckpoint = {
  authorization: LocationState['status'];
  backgroundCapture: LocationState['backgroundCapture'];
  beforeCount: number;
  afterCount: number;
  persistedDelta: number;
  beforeSequence: number;
  afterSequence: number;
};
const LOCATION_BACKGROUND_DWELL_MS = 10_000;
// Native shutdown intent remains active for 30 seconds so delayed BLE
// link-supervision loss can still be classified as user initiated.
export const SHUTDOWN_NO_OP_WAIT_MS = 35_000;

export function shouldDeferPhysicalLocation(value = process.env.FLOAT_VIEW_DEFER_PHYSICAL_LOCATION) {
  return value === '1';
}

export async function runPhysicalConformance(
  target: PhysicalTarget,
  fixturePath: string,
  report: Progress = (message) => process.stdout.write(`${message}\n`)
) {
  const matrix = physicalScenarioMatrix[target.platform];
  const selected = process.env.FLOAT_VIEW_PHYSICAL_SCENARIOS?.split(',').filter(Boolean);
  const names: readonly PhysicalScenarioName[] = selected?.length
    ? selected.map((name) => {
        if (!matrix.includes(name as never))
          throw new Error(`Physical scenario ${name} is unavailable on ${target.platform}`);
        return name as PhysicalScenarioName;
      })
    : matrix;
  const results: Array<{
    scenario: PhysicalScenarioName;
    revision: number;
    durationMs: number;
    writePayloadMaximum: number;
    locationCheckpoint?: LocationCheckpoint;
  }> = [];
  let primaryFailure: unknown;
  report(`Physical conformance: ${target.name}`);
  report(`Preparing ${target.name}...`);
  try {
    await target.prepareSuite?.((message) => report(`  ${message}`));
    await target.prepareLocation?.((message) => report(`  ${message}`));
    report(`Prepared ${target.name}`);
    for (const [index, name] of names.entries()) {
      report('');
      report(`[${index + 1}/${names.length}] ${name}`);
      const started = performance.now();
      try {
        const result = await runPhysicalScenario(target, fixturePath, name, report);
        const durationMs = performance.now() - started;
        assert.ok(
          result.writePayloadMaximum <= (target.platform === 'android' ? 20 : 244),
          'Native write exceeded platform payload ceiling'
        );
        results.push({ ...result, durationMs });
        report(`✓ ${name} (${formatDuration(durationMs)})`);
      } catch (error) {
        report(`✗ ${name} (${formatDuration(performance.now() - started)})`);
        throw error;
      }
    }
    const result = {
      target: target.name,
      fixturePath,
      refloatVersion: process.env.FLOAT_VIEW_PHYSICAL_REFLOAT_VERSION ?? null,
      generatedAt: new Date().toISOString(),
      scenarios: results,
    };
    const reportPath = process.env.FLOAT_VIEW_PHYSICAL_REPORT;
    if (reportPath) {
      mkdirSync(dirname(reportPath), { recursive: true });
      writeFileSync(reportPath, `${JSON.stringify(result, null, 2)}\n`);
    }
    return result;
  } catch (error) {
    primaryFailure = error;
    throw error;
  } finally {
    report('');
    report(`Cleaning up ${target.name}...`);
    try {
      await target.finishSuite?.((message) => report(`  ${message}`));
      report(`Cleaned up ${target.name}`);
    } catch (cleanupError) {
      if (primaryFailure) {
        process.stderr.write(`Cleanup also failed: ${errorMessage(cleanupError)}\n`);
      } else {
        throw cleanupError;
      }
    }
  }
}

async function runPhysicalScenario(
  target: PhysicalTarget,
  fixturePath: string,
  name: PhysicalScenarioName,
  report: Progress
) {
  const scenario = physicalScenario(name, process.env.FLOAT_VIEW_PHYSICAL_REFLOAT_VERSION);
  const fixture = await FixtureClient.open(fixturePath);
  const runner = new FixtureRunner(scenario, fixture, undefined, 1_000, (message) => report(`  ${message}`));
  let client: ConformanceClient | undefined;
  let passed = false;
  let fixtureStopped = false;
  let locationCheckpoint: LocationCheckpoint | undefined;
  try {
    await runner.start();
    const endpoint = await target.start();
    client = await ConformanceClient.open(endpoint.url, endpoint.origin, 10_000);
    const starting = await client.request('state.get', {});
    const bluetoothReady =
      starting.bluetooth.radio === 'powered_on'
        ? starting
        : await client.waitForState((state) => state.bluetooth.radio === 'powered_on', {
            afterRevision: starting.revision,
            timeoutMs: 15_000,
          });
    assert.equal(bluetoothReady.bluetooth.radio, 'powered_on');
    await client.request('bluetooth.requestPermission', {}, 15_000);
    const initial = await client.request('state.get', {});
    await client.request('settings.update', {
      expectedRevision: initial.settings.revision,
      patch: { minimumSessionDistance: 0, locationRecordingEnabled: false },
    });
    const discovered = await discoverFixture(client, scenario.board.advertisedName, initial.revision);
    const device = discovered.discoveredDevices.find((candidate) => candidate.name === scenario.board.advertisedName);
    assert.ok(device, `Did not discover ${scenario.board.advertisedName}`);
    let beforeConnect = discovered;
    if ('preferredCanId' in scenario) {
      const saved = discovered.savedDevices.find(
        (candidate) =>
          candidate.advertisedName === scenario.board.advertisedName ||
          candidate.bluetoothIdentifier === device.deviceId
      );
      if (!saved && scenario.preferredCanId !== null) {
        throw new Error('Explicit-CAN physical scenario requires the identity scenario first');
      }
      if (saved) {
        const profile = await client.request('boardProfile.get', {
          savedDeviceId: saved.savedDeviceId,
        });
        await client.request('boardProfile.update', {
          savedDeviceId: saved.savedDeviceId,
          expectedRevision: profile.revision,
          patch: { preferredCanId: scenario.preferredCanId ?? null },
        });
        beforeConnect = await client.request('state.get', {});
      }
    }
    await client.request(
      'device.connect',
      { deviceId: device.deviceId, trackingMode: name === 'guest' ? 'guest' : 'saved' },
      15_000
    );
    let afterConnectRevision = beforeConnect.revision;
    if (scenario.selectCanId !== undefined) {
      const selecting = await client.waitForState(
        (state) =>
          state.activeConnection?.deviceId === device.deviceId &&
          state.activeConnection.state === 'selecting_controller',
        { afterRevision: afterConnectRevision, timeoutMs: 20_000 }
      );
      assert.deepEqual(
        selecting.activeConnection!.controllerCandidates.map((candidate) => candidate.canId),
        [7, 42]
      );
      assert.deepEqual(
        selecting.activeConnection!.controllerCandidates.map((candidate) => candidate.vescSupported),
        [false, true]
      );
      await client.request('device.selectController', {
        deviceId: device.deviceId,
        canId: scenario.selectCanId,
      });
      afterConnectRevision = selecting.revision;
    }
    if (name === 'bms-standalone') {
      const ready = await client.waitForState(
        (state) =>
          state.activeConnection?.deviceId === device.deviceId &&
          state.activeConnection.state === 'ready' &&
          state.activeConnection.deviceKind === 'bms' &&
          state.activeConnection.bms.status === 'attached',
        { afterRevision: afterConnectRevision, timeoutMs: 20_000 }
      );
      const connection = ready.activeConnection!;
      assert.equal(connection.firmware, null);
      assert.equal(connection.controllerConnection, null);
      assert.deepEqual(connection.controllerCandidates, []);
      assert.equal(connection.connectionIssue, null);
      assert.deepEqual(connection.bms.connection, { kind: 'direct' });
      assert.equal(connection.bms.latestSnapshot?.reportedCanId, physicalDirectReportedCanId(scenario));
      assert.equal(connection.refloatControls, null);
      assert.deepEqual(connection.controllerActions, { shutdownAvailable: false, pendingAction: null });
      assert.equal(connection.activeSession, null);
      assert.equal(connection.latestSample, null);
      assert.equal(connection.latestLocation, null);
      assert.equal(connection.currentLimits, null);

      await runner.waitForRequests((requests) => requests.some((request) => request.kind === 'bms-values'), 10_000);
      const requestKinds = runner.simulator.requests.map((request) => request.kind);
      assert.ok(requestKinds.includes('transport-firmware'));
      assert.ok(requestKinds.includes('bms-values'));
      assert.ok(!requestKinds.includes('can-ping'));
      assert.ok(!requestKinds.includes('controller-firmware'));
      assert.ok(!requestKinds.includes('motor-configuration'));
      assert.ok(!requestKinds.includes('refloat-info'));
      assert.ok(!requestKinds.includes('refloat-layout'));
      assert.ok(!requestKinds.includes('refloat-control'));
      assert.ok(!requestKinds.includes('controller-shutdown'));
      assert.ok(!requestKinds.includes('telemetry'));

      const beforeDisconnect = await client.request('state.get', {});
      await client.request('device.disconnect', {}, 10_000);
      await client.waitForState((state) => state.activeConnection === null, {
        afterRevision: beforeDisconnect.revision,
        timeoutMs: 15_000,
      });
      runner.verify();
      const finalState = await client.request('state.get', {});
      passed = true;
      return {
        scenario: name,
        revision: finalState.revision,
        locationCheckpoint,
        writePayloadMaximum: runner.writePayloadMaximum,
      };
    }
    let ready = await client.waitForState(
      (state) =>
        state.activeConnection?.deviceId === device.deviceId &&
        state.activeConnection.state === 'ready' &&
        state.activeConnection.firmware !== null,
      { afterRevision: afterConnectRevision, timeoutMs: 20_000 }
    );
    if (scenario.expectedBmsConnection && ready.activeConnection?.bms.status !== 'attached') {
      ready = await client.waitForState(
        (state) =>
          state.activeConnection?.deviceId === device.deviceId && state.activeConnection.bms.status === 'attached',
        { afterRevision: ready.revision, timeoutMs: 15_000 }
      );
    }
    assertIdentity(ready.activeConnection!.firmware!, scenario.board, ready.activeConnection!.controllerConnection);
    const route = ready.activeConnection!.controllerConnection;
    assert.ok(route, 'Ready connection did not report its controller route');
    if (scenario.board.transport.hardwareType === 0) {
      assert.deepEqual(route, { kind: 'direct' });
    } else {
      assert.equal(route.kind, 'can');
      if (route.kind === 'can') {
        assert.equal(route.canId, scenario.selectCanId ?? scenario.board.controllers[0]?.canId);
      }
    }
    assert.deepEqual(ready.activeConnection!.controllerCandidates, []);
    assert.equal(ready.activeConnection!.deviceKind, 'board');
    assert.equal(ready.activeConnection!.connectionIssue, null);
    if (scenario.expectedBmsConnection === 'direct') {
      assert.deepEqual(ready.activeConnection!.bms.connection, { kind: 'direct' });
    } else if (scenario.expectedBmsConnection) {
      assert.deepEqual(ready.activeConnection!.bms.connection, {
        kind: 'can',
        canId: scenario.expectedBmsConnection.canId,
      });
    }
    if (scenario.expectedBmsConnection) {
      assert.equal(ready.activeConnection!.bms.status, 'attached');
      assert.equal(
        ready.activeConnection!.bms.latestSnapshot?.reportedCanId,
        scenario.expectedBmsConnection === 'direct'
          ? physicalDirectReportedCanId(scenario)
          : scenario.expectedBmsConnection.canId
      );
    }
    if (name === 'guest') {
      assert.equal(ready.activeConnection?.activeSession, null);
      assert.equal(ready.sessionCatalogRevision, initial.sessionCatalogRevision);
      assert.deepEqual(ready.savedDevices, beforeConnect.savedDevices);
      await runner.waitForRequests(
        (requests) => requests.filter((request) => request.kind === 'telemetry').length >= 5,
        15_000
      );
      const sample = await client.waitForState(
        (state) => state.activeConnection?.latestSample?.rideState === 'riding',
        {
          afterRevision: ready.revision,
          timeoutMs: 15_000,
        }
      );
      assert.equal(sample.activeConnection?.activeSession, null);
      assert.equal(sample.sessionCatalogRevision, initial.sessionCatalogRevision);
      await client.request('device.disconnect', {}, 10_000);
      const finalState = await client.waitForState((state) => state.activeConnection === null, {
        afterRevision: sample.revision,
        timeoutMs: 15_000,
      });
      assert.equal(finalState.sessionCatalogRevision, initial.sessionCatalogRevision);
      runner.verify();
      passed = true;
      return {
        scenario: name,
        revision: finalState.revision,
        locationCheckpoint,
        writePayloadMaximum: runner.writePayloadMaximum,
      };
    }
    const activeSession = ready.activeConnection?.activeSession;
    assert.ok(activeSession, 'Ready connection did not create an active provisional session');

    if (name === 'control-direct' || name === 'control-can') {
      await exerciseRefloatConfiguration(client, ready, true);
      await runRefloatControlCheckpoint(client, runner, ready, report);
      const beforeDisconnect = await client.request('state.get', {});
      await client.request('device.disconnect', {}, 10_000);
      await client.waitForState((state) => state.activeConnection === null, {
        afterRevision: beforeDisconnect.revision,
        timeoutMs: 15_000,
      });
    } else if (name === 'shutdown-direct' || name === 'shutdown-can' || name === 'shutdown-no-op') {
      await runControllerShutdownCheckpoint(client, runner, ready, name === 'shutdown-no-op');
    } else if (name === 'ride') {
      await runRide(client, runner, device.deviceId, activeSession.sessionId);
    } else if (name === 'unexpected-disconnect') {
      await runReconnect(client, runner, device.deviceId, activeSession.sessionId, ready.revision, false);
    } else if (name === 'background') {
      const backgroundResult = await runBackground(
        target,
        client,
        runner,
        device.deviceId,
        activeSession.sessionId,
        report
      );
      client = backgroundResult.client;
      locationCheckpoint = backgroundResult.locationCheckpoint;
    } else {
      // The fixture's fifth telemetry response is the first riding sample.
      // Every session expected to remain reviewable must reach it explicitly.
      await runner.waitForRequests(
        (requests) =>
          requests.filter((request) => request.kind === 'telemetry').length >=
          Math.max(5, scenario.minimumTelemetryResponses),
        15_000
      );
      // The legacy Refloat wire family fixture intentionally exercises only
      // its fixed idle packet. It still proves readiness, polling, and session
      // persistence, but cannot produce the dynamic riding transition used by
      // the other physical scenarios.
      if (scenario.board.controllers[0]?.refloatVersion !== '1.0') {
        await client.waitForState((state) => state.activeConnection?.latestSample?.rideState === 'riding', {
          afterRevision: ready.revision,
          timeoutMs: 15_000,
        });
      }
      if (scenario.board.controllers[0]?.refloatVersion === '1.0') {
        const beforeDisconnect = await client.request('state.get', {});
        await client.request('device.disconnect', {}, 10_000);
        await client.waitForState((state) => state.activeConnection === null, {
          afterRevision: beforeDisconnect.revision,
          timeoutMs: 15_000,
        });
      } else {
        await completeSession(client, device.deviceId, activeSession.sessionId);
      }
    }

    if ((name === 'ride' || name === 'background') && target.verifyFilePresentation) {
      runner.verify();
      await runner.stop(true);
      fixtureStopped = true;
      await target.verifyFilePresentation();
      report('  Native file presentation appeared and cancellation completed');
    }
    runner.verify();
    const finalState = await client.request('state.get', {});
    passed = true;
    return {
      scenario: name,
      revision: finalState.revision,
      locationCheckpoint,
      writePayloadMaximum: runner.writePayloadMaximum,
    };
  } catch (error) {
    const diagnostics = runner.diagnosticSnapshot();
    if (diagnostics.length) {
      process.stderr.write(`Fixture diagnostics (${name}):\n${diagnostics.join('\n')}\n`);
    }
    throw error;
  } finally {
    client?.close();
    await target.stop().catch(() => undefined);
    if (!fixtureStopped) await runner.stop(passed);
  }
}

async function runRefloatControlCheckpoint(
  client: ConformanceClient,
  runner: FixtureRunner,
  ready: AppState,
  report: Progress
) {
  let state = ready;
  if (
    state.activeConnection?.refloatControls?.status !== 'ready' ||
    state.activeConnection?.latestSample?.rideState !== 'idle'
  ) {
    state = await client.waitForState(
      (value) =>
        value.activeConnection?.refloatControls?.status === 'ready' &&
        value.activeConnection.latestSample?.rideState === 'idle',
      { afterRevision: state.revision, timeoutMs: 15_000 }
    );
  }
  const initial = state.activeConnection!.refloatControls!;
  assert.equal(initial.writable, true);
  assert.equal(initial.pendingAction, null);
  const requestOffset = runner.simulator.requests.length;

  const set = async (control: 'package' | 'flywheel' | 'leds' | 'headlights', enabled: boolean) => {
    report(`  Control: ${control}=${enabled}`);
    const result = await client.request('refloat.control.set', { control, enabled }, 10_000);
    assert.equal(result.pendingAction, null, `${control} did not confirm`);
    const field = `${control}Enabled` as 'packageEnabled' | 'flywheelEnabled' | 'ledsEnabled' | 'headlightsEnabled';
    assert.equal(result[field], enabled, `${control} confirmation did not match the request`);
  };

  await set('leds', !initial.ledsEnabled!);
  await set('headlights', !initial.headlightsEnabled!);
  await set('package', false);
  await set('package', true);
  await set('flywheel', true);
  await set('flywheel', false);
  if (initial.fatalErrorClearable) {
    report('  Control: clear fatal latch');
    const cleared = await client.request('refloat.fatalError.clear', {}, 10_000);
    assert.equal(cleared.fatalErrorLatched, false);
    assert.equal(cleared.pendingAction, null);
  }

  const controlRequests = runner.simulator.requests.slice(requestOffset);
  assert.ok(controlRequests.filter((request) => request.kind === 'refloat-control').length >= 6);
  assert.ok(
    controlRequests.every(
      (request) =>
        request.kind === 'refloat-control' ||
        request.kind === 'telemetry' ||
        request.kind === 'refloat-configuration-current'
    ),
    'control checkpoint emitted a request other than control, telemetry, or the safe post-flywheel configuration refresh'
  );
}

async function runControllerShutdownCheckpoint(
  client: ConformanceClient,
  runner: FixtureRunner,
  ready: AppState,
  expectNoOp: boolean
) {
  const state =
    ready.activeConnection?.controllerActions.shutdownAvailable === true
      ? ready
      : await client.waitForState((value) => value.activeConnection?.controllerActions.shutdownAvailable === true, {
          afterRevision: ready.revision,
          timeoutMs: 15_000,
        });
  const requestOffset = runner.simulator.requests.length;
  assert.deepEqual(await client.request('controller.shutdown', {}, 10_000), { accepted: true });
  await runner.waitForRequests(
    (requests) => requests.slice(requestOffset).some((request) => request.kind === 'controller-shutdown'),
    10_000
  );
  const writes = runner.simulator.requests.slice(requestOffset);
  assert.equal(writes.filter((request) => request.kind === 'controller-shutdown').length, 1);
  // A single read-only schema page can already be in the BLE stack when the
  // user action wins scheduler priority. It may finish after requestOffset,
  // but discovery must not continue and no other command is permitted.
  assert.ok(
    writes.filter((request) => request.kind === 'refloat-configuration-schema').length <= 1,
    'shutdown checkpoint continued auxiliary configuration discovery'
  );
  assert.ok(
    writes.every(
      (request) =>
        request.kind === 'controller-shutdown' ||
        request.kind === 'telemetry' ||
        request.kind === 'refloat-configuration-schema'
    ),
    'shutdown checkpoint emitted a non-shutdown/non-telemetry/non-schema-read request; motor movement is forbidden'
  );
  if (expectNoOp) {
    const continued = await client.waitForState(
      (value) =>
        value.activeConnection?.controllerActions.pendingAction === null &&
        value.activeConnection.controllerActions.shutdownAvailable,
      { afterRevision: state.revision, timeoutMs: SHUTDOWN_NO_OP_WAIT_MS }
    );
    await client.request('device.disconnect', {}, 10_000);
    await client.waitForState((value) => value.activeConnection === null, {
      afterRevision: continued.revision,
      timeoutMs: 15_000,
    });
  } else {
    await client.waitForState((value) => value.activeConnection === null, {
      afterRevision: state.revision,
      timeoutMs: 15_000,
    });
  }
}

function physicalDirectReportedCanId(scenario: (typeof fixtureScenarios)[PhysicalScenarioName]) {
  return scenario.board.localBms?.values.reportedCanId ?? 0;
}

async function discoverFixture(client: ConformanceClient, advertisedName: string, afterRevision: number) {
  let latestRevision = afterRevision;
  let lastError: unknown;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    await client.request('bluetooth.scan', { timeoutMs: 5_000 }, 10_000);
    try {
      return await client.waitForState(
        (state) => state.discoveredDevices.some((device) => device.name === advertisedName),
        { afterRevision: latestRevision, timeoutMs: 10_000 }
      );
    } catch (error) {
      lastError = error;
      latestRevision = (await client.request('state.get', {})).revision;
    }
  }
  throw lastError ?? new Error(`Did not discover ${advertisedName}`);
}

async function runRide(
  client: ConformanceClient,
  runner: FixtureRunner,
  deviceId: string,
  sessionId: string,
  exerciseReview = true
) {
  await runner.waitForRequests(
    (requests) =>
      requests.filter((request) => request.kind === 'telemetry').length >= runner.scenario.minimumTelemetryResponses,
    20_000
  );
  await completeSession(client, deviceId, sessionId, exerciseReview);
}

async function runReconnect(
  client: ConformanceClient,
  runner: FixtureRunner,
  deviceId: string,
  sessionId: string,
  afterRevision: number,
  exerciseReview = true
) {
  const reconnecting = client.waitForState((state) => state.activeConnection?.state === 'reconnecting', {
    afterRevision,
    timeoutMs: 20_000,
  });
  await runner.waitForRequests(
    (requests) =>
      requests.filter((request) => request.kind === 'telemetry').length >= runner.scenario.minimumTelemetryResponses,
    25_000
  );
  const reconnectingState = await reconnecting;
  const restored = await client.waitForState(
    (state) =>
      state.activeConnection?.deviceId === deviceId &&
      state.activeConnection.state === 'ready' &&
      state.activeConnection.firmware !== null,
    { afterRevision: reconnectingState.revision, timeoutMs: 20_000 }
  );
  assert.ok(
    restored.activeConnection?.activeSession?.sessionId === sessionId &&
      restored.activeConnection.activeSession.state === 'active',
    'Reconnect did not preserve the active session'
  );
  await completeSession(client, deviceId, sessionId, exerciseReview);
}

async function runBackground(
  target: PhysicalTarget,
  client: ConformanceClient,
  runner: FixtureRunner,
  deviceId: string,
  sessionId: string,
  report: Progress
): Promise<{ client: ConformanceClient; locationCheckpoint: LocationCheckpoint }> {
  assert.ok(target.background && target.foreground && target.endpoint, 'Mobile lifecycle is unavailable');
  const deferPhysicalLocation = shouldDeferPhysicalLocation();
  const location = deferPhysicalLocation
    ? (await client.request('state.get', {})).location
    : await enablePhysicalLocation(client, report);
  if (deferPhysicalLocation) {
    report('  Location: automated authorization and acquisition deferred to manual verification');
  }
  await runner.waitForRequests(
    (requests) => requests.filter((request) => request.kind === 'telemetry').length >= 4,
    15_000
  );
  const before = await client.request('state.get', {});
  const beforeSession = before.activeConnection?.activeSession;
  assert.ok(beforeSession, 'Active session disappeared before backgrounding');
  const beforeMetadata = await client.request('session.get', { sessionId });
  report('  App > background');
  await target.background();
  client.close();
  await runner.waitForRequests(
    (requests) =>
      requests.filter((request) => request.kind === 'telemetry').length >= runner.scenario.minimumTelemetryResponses,
    20_000
  );
  report(`  Location: background dwell ${LOCATION_BACKGROUND_DWELL_MS}ms`);
  await new Promise((resolve) => setTimeout(resolve, LOCATION_BACKGROUND_DWELL_MS));
  report('  App > foreground');
  await target.foreground();
  const endpoint = await target.endpoint();
  const replacement = await ConformanceClient.open(endpoint.url, endpoint.origin, 15_000);
  const recovered = await replacement.request('state.get', {});
  assert.equal(recovered.activeConnection?.deviceId, deviceId);
  const recoveredSession = recovered.activeConnection?.activeSession;
  assert.ok(recoveredSession, 'Foregrounded frontend did not recover the active session');
  assert.ok(
    recoveredSession.sampleCount > beforeSession.sampleCount,
    'Native session did not accumulate samples while backgrounded'
  );
  const afterMetadata = await replacement.request('session.get', { sessionId });
  const persistedDelta = afterMetadata.location.count - beforeMetadata.location.count;
  const locationCheckpoint: LocationCheckpoint = {
    authorization: location.status,
    backgroundCapture: location.backgroundCapture,
    beforeCount: beforeMetadata.location.count,
    afterCount: afterMetadata.location.count,
    persistedDelta,
    beforeSequence: beforeMetadata.snapshot.committedLocationSequence,
    afterSequence: afterMetadata.snapshot.committedLocationSequence,
  };
  report(
    `  Location checkpoint: status=${location.status} background=${location.backgroundCapture} ` +
      `rows=${beforeMetadata.location.count}->${afterMetadata.location.count} ` +
      `sequence=${beforeMetadata.snapshot.committedLocationSequence}->${afterMetadata.snapshot.committedLocationSequence}`
  );
  assert.ok(
    afterMetadata.snapshot.committedLocationSequence >= beforeMetadata.snapshot.committedLocationSequence,
    'Location boundary regressed after foreground recovery'
  );
  if (!deferPhysicalLocation) {
    assert.ok(persistedDelta > 0, 'No physical location observation persisted during background dwell');
  }
  await completeSession(replacement, deviceId, sessionId, true);
  return { client: replacement, locationCheckpoint };
}

async function enablePhysicalLocation(client: ConformanceClient, report: Progress) {
  let state = await client.request('state.get', {});
  if (!state.settings.locationRecordingEnabled) {
    const settings = await client.request('settings.update', {
      expectedRevision: state.settings.revision,
      patch: { locationRecordingEnabled: true },
    });
    assert.equal(settings.locationRecordingEnabled, true);
  }

  let location = await client.request('location.requestPermission', {});
  if (location.status === 'notDetermined') {
    location = await pollLocationState(client, (next) => next.status !== 'notDetermined', 60_000);
  }
  if (location.status === 'foregroundOnly' || location.permissionAction === 'openSettings') {
    report('  Location: background capability requires platform settings; waiting for authorization');
    await client.request('location.openSettings', {});
    location = await pollLocationState(
      client,
      (next) => next.status === 'authorized' && next.backgroundCapture === 'available',
      60_000
    );
  }
  assert.equal(location.status, 'authorized', `Location authorization is ${location.status}`);
  assert.equal(location.backgroundCapture, 'available', `Location background capture is ${location.backgroundCapture}`);
  report(`  Location: ${location.status}/${location.backgroundCapture}`);
  return location;
}

async function pollLocationState(
  client: ConformanceClient,
  predicate: (state: LocationState) => boolean,
  timeoutMs: number
) {
  const deadline = performance.now() + timeoutMs;
  let latest = (await client.request('state.get', {})).location;
  while (performance.now() < deadline) {
    if (predicate(latest)) return latest;
    await new Promise((resolve) => setTimeout(resolve, 250));
    latest = (await client.request('state.get', {})).location;
  }
  throw new Error(
    `Timed out resolving location state; latest=${latest.status}/${latest.backgroundCapture} reason=${latest.reason ?? 'none'}`
  );
}

function formatDuration(milliseconds: number) {
  return `${(milliseconds / 1_000).toFixed(1)}s`;
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

async function completeSession(client: ConformanceClient, deviceId: string, sessionId: string, exerciseReview = false) {
  const before = await client.request('state.get', {});
  await client.request('device.disconnect', {}, 10_000);
  const disconnected = await client.waitForState((state) => state.activeConnection === null, {
    afterRevision: before.revision,
    timeoutMs: 15_000,
  });
  assert.ok(!disconnected.activeConnection || disconnected.activeConnection.deviceId !== deviceId);
  if (!exerciseReview) return;
  const completed = await client.request('state.get', {});
  const savedDevice = completed.savedDevices.find((item) => item.bluetoothIdentifier === deviceId);
  assert.ok(savedDevice);
  const page = await client.request('session.list', { savedDeviceId: savedDevice.savedDeviceId, limit: 100 });
  const session = page.items.find((item) => item.sessionId === sessionId);
  assert.ok(session, 'Completed durable session is absent');
  assert.equal(session.state, 'completed');
  assert.ok(session.sampleCount > 0, 'Ride session persisted no samples');
  assert.ok(session.ridingElapsedMs > 0, 'Ride session recorded no riding time');
  const detail = await client.request('session.get', {
    sessionId,
  });
  assert.ok(detail.fields.length > 0);
  assert.ok(detail.snapshot.committedElapsedMs >= 0);
  const field = detail.fields[0]?.name;
  assert.ok(field);
  const data = await client.sessionData({
    kind: 'series',
    sessionId,
    snapshot: detail.snapshot,
    range: { startElapsedMs: 0, endElapsedMs: detail.snapshot.committedElapsedMs },
    fields: [field],
    buckets: 8,
    gapPolicy: { thresholdSeconds: null },
  });
  assert.equal(data.kind, 'series');
  await client.request('session.export', { sessionId, format: 'csv' });
  await client.request('session.delete', { sessionId });
  const deleted = await client.request('session.list', { savedDeviceId: savedDevice.savedDeviceId, limit: 100 });
  assert.ok(!deleted.items.some((item) => item.sessionId === sessionId));
}

function assertIdentity(
  firmware: {
    vescVersion: string;
    packageName: string;
    packageVersion: string;
    hardwareName?: string | null;
    supported: boolean;
  },
  board: (typeof fixtureScenarios)['identity']['board'],
  route: { kind: 'direct' } | { kind: 'can'; canId: number } | null
) {
  const controller =
    route?.kind === 'can'
      ? board.controllers.find((candidate) => candidate.canId === route.canId)
      : board.controllers[0];
  assert.ok(controller, 'Reported route has no fixture controller');
  assert.equal(
    firmware.vescVersion,
    `${controller.firmware.major}.${String(controller.firmware.minor).padStart(2, '0')}`
  );
  assert.equal(firmware.packageName, 'refloat');
  assert.equal(firmware.packageVersion, controller.refloatVersion);
  assert.equal(firmware.hardwareName, controller.firmware.hardwareName);
  assert.equal(firmware.supported, true);
}
