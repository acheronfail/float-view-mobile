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
  test(
    `${target.name}: recorded loads respect available capture-time controller limits`,
    { timeout: 60_000 },
    async () => {
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
        const state = await client.waitForState((state) =>
          Boolean(state.activeConnection?.latestSample && state.activeConnection?.activeSession?.sessionId)
        );
        const connection = state.activeConnection!;
        // Wait for multiple observations to reach the durable session boundary.
        await client.waitForState(
          (next) => (next.activeConnection?.latestSample?.sequence ?? 0) > connection.latestSample!.sequence + 3
        );
        const metadata = await client.request('session.get', { sessionId: connection.activeSession?.sessionId! });
        if (!connection.currentLimits) {
          assert.ok(!metadata.fields.some((field) => field.name === 'motorLoad' || field.name === 'batteryLoad'));
          return;
        }
        for (const name of ['motorLoad', 'batteryLoad']) {
          assert.ok(metadata.fields.some((field) => field.name === name && field.source === 'extension'));
        }
        const fields = [
          'motorLoad',
          'batteryLoad',
          'motorCurrentA',
          'filteredMotorCurrentA',
          'directionalMotorCurrentA',
          'batteryCurrentA',
        ];
        const samples = await client.sessionData({
          kind: 'samples',
          sessionId: connection.activeSession?.sessionId!,
          snapshot: metadata.snapshot,
          range: { startElapsedMs: 0, endElapsedMs: metadata.snapshot.committedElapsedMs },
          fields,
          mode: 'window',
          limit: 100,
        });
        assert.equal(samples.kind, 'samples');
        const limits = connection.currentLimits!;
        const columns = samples.values;
        assert.ok(columns.motorLoad!.some((value) => value !== null));
        for (let index = 0; index < columns.motorLoad!.length; index++) {
          const raw = columns.motorCurrentA![index] ?? null;
          const motor = columns.filteredMotorCurrentA![index] ?? columns.directionalMotorCurrentA![index] ?? raw;
          const battery = columns.batteryCurrentA![index] ?? null;
          const motorLimit = raw === null ? null : raw < 0 ? limits.motorCurrentMinA : limits.motorCurrentMaxA;
          const batteryLimit =
            battery === null ? null : battery < 0 ? limits.batteryCurrentMinA : limits.batteryCurrentMaxA;
          assert.equal(
            columns.motorLoad![index],
            motor === null || !motorLimit ? null : Math.min(Math.abs(motor / motorLimit), 1)
          );
          assert.equal(
            columns.batteryLoad![index],
            battery === null || !batteryLimit ? null : Math.min(Math.abs(battery / batteryLimit), 1)
          );
        }
      } finally {
        client?.close();
        await target.stop();
      }
    }
  );
}
