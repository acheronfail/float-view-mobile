import assert from 'node:assert/strict';
import { test } from 'node:test';
import { makeDirectTarget } from '../targets/direct.ts';
import { ConformanceClient } from '../src/client.ts';
import { getMockBootstrap, startMockServer, stopMockServer } from '../../frontend/backend-mock/index.ts';

test(
  'direct: sessions capture the service estimate and preserve controller percentage',
  { timeout: 60_000 },
  async () => {
    const target = makeDirectTarget();
    let client: ConformanceClient | undefined;
    try {
      const endpoint = await target.start();
      client = await ConformanceClient.open(endpoint.url, endpoint.origin);
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((s) => s.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const ready = await client.waitForState(
        (s) => !!s.activeConnection?.latestSample && !!s.activeConnection.activeSession
      );
      const id = ready.activeConnection!.savedDeviceId!;
      const profile = await client.request('boardProfile.get', { savedDeviceId: id });
      await client.request('boardProfile.update', {
        savedDeviceId: id,
        expectedRevision: profile.revision,
        patch: {
          batteryCellProfileId: 'generic-linear',
          seriesCount: ready.activeConnection!.currentLimits?.batterySeriesCells ?? 18,
          parallelCount: 2,
        },
      });
      const recorded = await client.waitForState(
        (s) =>
          s.activeConnection?.batteryEstimate.level != null &&
          s.activeConnection.latestSample!.sequence > ready.activeConnection!.latestSample!.sequence + 2
      );
      const active = recorded.activeConnection!;
      const metadata = await client.request('session.get', { sessionId: active.activeSession!.sessionId });
      assert.ok(metadata.fields.some((f) => f.name === 'estimatedBatteryLevel' && f.source === 'extension'));
      const samples = await client.sessionData({
        kind: 'samples',
        sessionId: metadata.summary.sessionId,
        snapshot: metadata.snapshot,
        range: { startElapsedMs: 0, endElapsedMs: metadata.snapshot.committedElapsedMs },
        fields: ['batteryLevel', 'estimatedBatteryLevel'],
        mode: 'window',
        limit: 100,
      });
      assert.equal(samples.kind, 'samples');
      const index = samples.sequence.indexOf(active.latestSample!.sequence);
      assert.ok(index >= 0);
      assert.equal(samples.values.estimatedBatteryLevel![index], active.batteryEstimate.level);
      assert.equal(samples.values.batteryLevel![index], active.latestSample!.batteryLevel);
    } finally {
      client?.close();
      await target.stop();
    }
  }
);

test('mock: session data carries distinct controller and estimated battery fields', async () => {
  let client: ConformanceClient | undefined;
  try {
    const endpoint = getMockBootstrap(await startMockServer(0));
    client = await ConformanceClient.open(endpoint.webSocketUrl, endpoint.origin);
    const metadata = await client.request('session.get', { sessionId: 'session-1' });
    const samples = await client.sessionData({
      kind: 'samples',
      sessionId: 'session-1',
      snapshot: metadata.snapshot,
      range: { startElapsedMs: 0, endElapsedMs: 60_000 },
      fields: ['batteryLevel', 'estimatedBatteryLevel'],
      mode: 'window',
      limit: 100,
    });
    assert.equal(samples.kind, 'samples');
    assert.equal(samples.values.batteryLevel![0], 0.68);
    assert.equal(samples.values.estimatedBatteryLevel![0], 0.61);
  } finally {
    client?.close();
    await stopMockServer();
  }
});
