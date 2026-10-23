import assert from 'node:assert/strict';
import { test } from 'node:test';
import { makeDirectTarget } from '../targets/direct.ts';
import { getMockBootstrap, startMockServer, stopMockServer } from '../../frontend/backend-mock/index.ts';
import { ConformanceClient } from '../src/client.ts';

const mockTarget = {
  name: 'mock',
  async start() {
    const endpoint = getMockBootstrap(await startMockServer(0));
    return { url: endpoint.webSocketUrl, origin: endpoint.origin };
  },
  stop: stopMockServer,
};

for (const target of [makeDirectTarget(), mockTarget]) {
  test(`${target.name}: session charts advertise recorded fields, including zero`, { timeout: 60_000 }, async () => {
    let client: ConformanceClient | undefined;
    try {
      const endpoint = await target.start();
      client = await ConformanceClient.open(endpoint.url, endpoint.origin);
      await client.request('bluetooth.requestPermission', {});
      await client.request('bluetooth.scan', { timeoutMs: 100 });
      const scanned = await client.waitForState((state) => state.discoveredDevices.length > 0);
      await client.request('device.connect', {
        deviceId: scanned.discoveredDevices[0]!.deviceId,
        trackingMode: 'saved',
      });
      const ready = await client.waitForState((state) =>
        Boolean(state.activeConnection?.activeSession && state.activeConnection.latestSample)
      );
      await client.waitForState(
        (state) =>
          (state.activeConnection?.latestSample?.sequence ?? 0) > ready.activeConnection!.latestSample!.sequence + 3
      );
      const metadata = await client.request('session.get', {
        sessionId: ready.activeConnection!.activeSession!.sessionId,
      });
      const fields = [
        'speedKph',
        'remoteInput',
        'packVoltageV',
        'atrAccelerationDifferenceErpmPerSecond',
        'atrSpeedBoost',
      ];
      const zeroField = target.name === 'direct' ? 'speedKph' : 'remoteInput';
      const samples = await client.sessionData({
        kind: 'samples',
        sessionId: metadata.summary.sessionId,
        snapshot: metadata.snapshot,
        range: { startElapsedMs: 0, endElapsedMs: metadata.snapshot.committedElapsedMs },
        fields,
        mode: 'window',
        limit: 100,
      });
      assert.equal(samples.kind, 'samples');
      assert.ok(
        samples.values[zeroField]!.some((value) => value === 0),
        'zero readings must survive recording'
      );
      for (const name of fields) {
        const recorded: boolean = samples.values[name]!.some((value) => value !== null);
        assert.equal(
          metadata.fields.some((field) => field.name === name),
          recorded,
          name
        );
      }
      if (target.name === 'direct') {
        for (const name of ['atrAccelerationDifferenceErpmPerSecond', 'atrSpeedBoost']) {
          assert.ok(!metadata.fields.some((field) => field.name === name), 'unavailable ATR charts must be omitted');
        }
      }
      // Range selection does not change session-wide field availability.
      const emptyRange = await client.sessionData({
        kind: 'samples',
        sessionId: metadata.summary.sessionId,
        snapshot: metadata.snapshot,
        range: {
          startElapsedMs: metadata.snapshot.committedElapsedMs + 1,
          endElapsedMs: metadata.snapshot.committedElapsedMs + 2,
        },
        fields: [zeroField],
        mode: 'window',
        limit: 100,
      });
      assert.equal(emptyRange.kind, 'samples');
      assert.deepEqual(emptyRange.values[zeroField], []);
      const refreshed = await client.request('session.get', { sessionId: metadata.summary.sessionId });
      assert.ok(refreshed.fields.some((field) => field.name === zeroField));
    } finally {
      client?.close();
      await target.stop();
    }
  });
}
