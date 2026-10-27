import assert from 'node:assert/strict';
import { test } from 'node:test';
import { setTimeout as pause } from 'node:timers/promises';
import { InteractiveTelemetrySource } from '../fixture/telemetry.ts';
import { ConformanceClient } from '../src/client.ts';
import { makeAndroidTarget } from '../targets/android.ts';
import { makeMacosTarget } from '../targets/macos.ts';

// Opt-in native software replay. Playback is observed after native phrase decoding,
// through the conformance-only OS driver; this never establishes physical audibility.
const selected = process.env.FLOAT_VIEW_NATIVE_REPLAY_HOST;
assert.ok(selected === undefined || selected === 'macos' || selected === 'android');
for (const host of ['macos', 'android'] as const) {
  if (selected && selected !== host) continue;
  for (const perCell of [false, true]) {
    test(
      `${host}: automatic battery replay with ${perCell ? 'per-cell' : 'pack'} limits`,
      { timeout: 180_000 },
      async () => {
        const telemetry = new InteractiveTelemetrySource();
        telemetry.setScene('idle');
        telemetry.pin('packageState', 'ready');
        telemetry.pin('rideState', 'idle');
        telemetry.pin('speedKph', 0);
        telemetry.pin('packVoltageV', 75);
        const observations: { phrase: string; at: number }[] = [];
        const replay = {
          telemetry,
          refloatVersion: perCell ? ('1.2.3' as const) : ('1.0.0' as const),
          onSpeech(phrase: string) {
            assert.ok(observations.length < 32, 'unbounded native speech');
            observations.push({ phrase, at: performance.now() });
          },
        };
        const target = host === 'macos' ? makeMacosTarget(false, replay) : makeAndroidTarget(undefined, replay);
        let client: ConformanceClient | undefined;
        try {
          const endpoint = await target.start();
          client = await ConformanceClient.open(endpoint.url, endpoint.origin);
          let state = await client.request('state.get', {});
          assert.equal(state.settings.announcements.batteryChargeEnabled, true);
          await client.request('settings.update', {
            expectedRevision: state.settings.revision,
            patch: { safetyDisclaimerAccepted: true },
          });
          await client.waitForState((s) => s.announcementAudio.status === 'ready', { timeoutMs: 10_000 });
          await client.request('bluetooth.requestPermission', {});
          const connect = async () => {
            await client!.request('bluetooth.scan', { timeoutMs: 10_000 });
            const scan = await client!.waitForState((s) => s.discoveredDevices.length > 0);
            await client!.request('device.connect', {
              deviceId: scan.discoveredDevices[0]!.deviceId,
              trackingMode: 'guest',
            });
            await client!.waitForState((s) => s.activeConnection?.refloatConfiguration?.currentLoaded === true, {
              timeoutMs: 20_000,
            });
          };
          await connect();
          let current = await client.request('refloat.configuration.read', { source: 'current', refresh: false });
          const changeLimits = async (low: number, high: number) => {
            current = await client!.request('refloat.configuration.write', {
              packageVersion: current.identity.packageVersion,
              schemaFingerprint: current.identity.schemaFingerprint,
              wireSignature: current.identity.wireSignature,
              baseRevision: current.revision,
              values: {
                ...current.values,
                tiltback_lv: { type: 'double', value: low },
                tiltback_hv: { type: 'double', value: high },
              },
            });
          };
          // Structurally readable but reversed limits must never supply guessed warnings.
          await changeLimits(perCell ? 4.3 : 86, perCell ? 3 : 60);
          telemetry.pin('packageState', 'running');
          telemetry.pin('rideState', 'riding');
          telemetry.pin('packVoltageV', 59);
          await client.waitForState(
            (s) =>
              s.activeConnection?.latestSample?.packageState === 'running' &&
              s.activeConnection.latestSample.packVoltageV === 59
          );
          await pause(2300);
          assert.equal(observations.length, 0, 'invalid limits triggered a native warning');
          telemetry.pin('packageState', 'ready');
          telemetry.pin('rideState', 'idle');
          telemetry.pin('packVoltageV', 75);
          await client.waitForState(
            (s) =>
              s.activeConnection?.latestSample?.packageState === 'ready' &&
              s.activeConnection.latestSample.packVoltageV === 75
          );
          await changeLimits(perCell ? 3 : 60, perCell ? 4.3 : 86);
          const received = async (voltage: number) => {
            telemetry.pin('packVoltageV', voltage);
            await client!.waitForState((s) => s.activeConnection?.latestSample?.packVoltageV === voltage);
          };
          const waitCount = async (count: number) => {
            const deadline = performance.now() + 8000;
            while (observations.length < count && performance.now() < deadline) await pause(25);
            assert.equal(observations.length, count, `native phrases: ${JSON.stringify(observations)}`);
            return observations[count - 1]!;
          };
          // No announcements in non-running state, even at the actual critical limit.
          await received(59);
          await pause(2300);
          assert.equal(observations.length, 0);
          // A brief advance low crossing must not consume an attempt.
          telemetry.pin('packageState', 'running');
          telemetry.pin('rideState', 'riding');
          await received(62);
          await pause(500);
          await received(75);
          await pause(1700);
          assert.equal(observations.length, 0);
          const lowStart = performance.now();
          await received(62);
          const low = await waitCount(1);
          assert.equal(low.phrase, 'Battery low');
          assert.ok(low.at - lowStart >= 1900, 'low confirmation bypassed');
          console.log(`${host}: native Battery low received`);
          // High entry is independent of the low cooldown, but requires regen.
          telemetry.pin('packageState', 'ready');
          telemetry.pin('rideState', 'idle');
          telemetry.pin('batteryCurrentA', -5);
          await received(85);
          await pause(1000);
          assert.equal(observations.length, 1);
          telemetry.pin('packageState', 'running');
          telemetry.pin('rideState', 'riding');
          telemetry.pin('batteryCurrentA', 5);
          await pause(1200);
          assert.equal(observations.length, 1);
          telemetry.pin('batteryCurrentA', -5);
          const high = await waitCount(2);
          assert.equal(high.phrase, 'Over voltage');
          assert.ok(high.at - low.at < 60_000, 'high incorrectly waited for low cooldown');
          console.log(`${host}: native Over voltage received`);
          if (perCell) {
            // Observe one full real monotonic interval in each native host.
            while (performance.now() < high.at + 59_700) {
              assert.equal(observations.length, 2, 'native warning repeated before one minute');
              await pause(250);
            }
            const repeat = await waitCount(3);
            assert.equal(repeat.phrase, 'Over voltage');
            assert.ok(repeat.at - high.at >= 59_900);
            console.log(`${host}: native one-minute repeat verified`);
          }
          const count = observations.length;
          state = await client.request('state.get', {});
          await client.request('settings.update', {
            expectedRevision: state.settings.revision,
            patch: { announcements: { ...state.settings.announcements, batteryChargeEnabled: false } },
          });
          await received(59);
          await client.request('device.disconnect', {});
          await connect();
          assert.equal((await client.request('state.get', {})).settings.announcements.batteryChargeEnabled, false);
          await pause(2300);
          assert.equal(observations.length, count, 'disabled native rule announced after reconnect');
          await received(85);
          state = await client.request('state.get', {});
          await client.request('settings.update', {
            expectedRevision: state.settings.revision,
            patch: { announcements: { ...state.settings.announcements, batteryChargeEnabled: true } },
          });
          await pause(1500);
          assert.equal(observations.length, count, 'reconnect or toggle bypassed native cooldown');
        } finally {
          client?.close();
          await target.stop();
        }
      }
    );
  }
}
