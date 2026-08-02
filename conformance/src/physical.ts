import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { FixtureRunner } from '../fixture/runner.ts';
import { fixtureScenarios, physicalScenarioMatrix, type PhysicalScenarioName } from '../fixture/scenarios.ts';
import { FixtureClient } from '../fixture/serial.ts';
import { ConformanceClient } from './client.ts';
import type { PhysicalTarget } from './suite.ts';

type Progress = (message: string) => void;

export async function runPhysicalConformance(
  target: PhysicalTarget,
  fixturePath: string,
  report: Progress = (message) => process.stdout.write(`${message}\n`)
) {
  const names = physicalScenarioMatrix[target.platform];
  const results: Array<{ scenario: PhysicalScenarioName; revision: number; durationMs: number }> = [];
  let primaryFailure: unknown;
  report(`Physical conformance: ${target.name}`);
  report(`Preparing ${target.name}...`);
  try {
    await target.prepareSuite?.((message) => report(`  ${message}`));
    report(`Prepared ${target.name}`);
    for (const [index, name] of names.entries()) {
      report('');
      report(`[${index + 1}/${names.length}] ${name}`);
      const started = performance.now();
      try {
        const result = await runPhysicalScenario(target, fixturePath, name, report);
        const durationMs = performance.now() - started;
        results.push({ ...result, durationMs });
        report(`✓ ${name} (${formatDuration(durationMs)})`);
      } catch (error) {
        report(`✗ ${name} (${formatDuration(performance.now() - started)})`);
        throw error;
      }
    }
    const result = { target: target.name, fixturePath, generatedAt: new Date().toISOString(), scenarios: results };
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
  const scenario = fixtureScenarios[name];
  const fixture = await FixtureClient.open(fixturePath);
  const runner = new FixtureRunner(scenario, fixture, undefined, 1_000, (message) => report(`  ${message}`));
  let client: ConformanceClient | undefined;
  let passed = false;
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
      patch: { minimumSessionDistance: 0 },
    });
    await client.request('bluetooth.scan', { timeoutMs: 5_000 }, 10_000);
    const discovered = await client.waitForState(
      (state) => state.discoveredDevices.some((device) => device.name === scenario.board.advertisedName),
      { afterRevision: initial.revision, timeoutMs: 15_000 }
    );
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
    await client.request('device.connect', { deviceId: device.deviceId }, 15_000);
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
    const ready = await client.waitForState(
      (state) =>
        state.activeConnection?.deviceId === device.deviceId &&
        state.activeConnection.state === 'ready' &&
        state.activeConnection.firmware !== null,
      { afterRevision: afterConnectRevision, timeoutMs: 20_000 }
    );
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
    assert.equal(ready.activeConnection!.connectionIssue, null);
    const activeSession = ready.recentSessions.find((session) => session.state === 'active');
    assert.ok(activeSession, 'Ready connection did not create an active provisional session');

    if (name === 'ride') {
      await runRide(client, runner, device.deviceId, activeSession.sessionId, target.platform !== 'android');
    } else if (name === 'unexpected-disconnect') {
      await runReconnect(client, runner, device.deviceId, activeSession.sessionId, ready.revision, false);
    } else if (name === 'background') {
      client = await runBackground(target, client, runner, device.deviceId, activeSession.sessionId, report);
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

    runner.verify();
    const finalState = await client.request('state.get', {});
    passed = true;
    return { scenario: name, revision: finalState.revision };
  } catch (error) {
    const diagnostics = runner.diagnosticSnapshot();
    if (diagnostics.length) {
      process.stderr.write(`Fixture diagnostics (${name}):\n${diagnostics.join('\n')}\n`);
    }
    throw error;
  } finally {
    client?.close();
    await target.stop().catch(() => undefined);
    await runner.stop(passed);
  }
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
    restored.recentSessions.some((session) => session.sessionId === sessionId && session.state === 'active'),
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
) {
  assert.ok(target.background && target.foreground && target.endpoint, 'Mobile lifecycle is unavailable');
  await runner.waitForRequests(
    (requests) => requests.filter((request) => request.kind === 'telemetry').length >= 4,
    15_000
  );
  const before = await client.request('state.get', {});
  const beforeSession = before.recentSessions.find((session) => session.sessionId === sessionId);
  assert.ok(beforeSession, 'Active session disappeared before backgrounding');
  report('  App > background');
  await target.background();
  client.close();
  await runner.waitForRequests(
    (requests) =>
      requests.filter((request) => request.kind === 'telemetry').length >= runner.scenario.minimumTelemetryResponses,
    20_000
  );
  report('  App > foreground');
  await target.foreground();
  const endpoint = await target.endpoint();
  const replacement = await ConformanceClient.open(endpoint.url, endpoint.origin, 15_000);
  const recovered = await replacement.request('state.get', {});
  assert.equal(recovered.activeConnection?.deviceId, deviceId);
  const recoveredSession = recovered.recentSessions.find((session) => session.sessionId === sessionId);
  assert.ok(recoveredSession, 'Foregrounded frontend did not recover the active session');
  assert.ok(
    recoveredSession.sampleCount > beforeSession.sampleCount,
    'Native session did not accumulate samples while backgrounded'
  );
  await completeSession(replacement, deviceId, sessionId, target.platform !== 'android');
  return replacement;
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
  const session = completed.recentSessions.find((item) => item.sessionId === sessionId);
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
  const deleted = await client.request('state.get', {});
  assert.ok(!deleted.recentSessions.some((item) => item.sessionId === sessionId));
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
