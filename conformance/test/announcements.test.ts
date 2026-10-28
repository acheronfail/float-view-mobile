import assert from 'node:assert/strict';
import { test } from 'node:test';
import { setTimeout as pause } from 'node:timers/promises';
import { makeDirectTarget, type SpeechEffect } from '../targets/direct.ts';
import { InteractiveTelemetrySource } from '../fixture/telemetry.ts';
import { ConformanceClient } from '../src/client.ts';

test(
  'direct: automatic footpads use wire telemetry, shared cadence and reconnect qualification',
  { timeout: 45_000 },
  async () => {
    const telemetry = new InteractiveTelemetrySource();
    telemetry.setScene('cruise');
    telemetry.pin('speedKph', 32);
    telemetry.pin('footpadState', 'right');
    telemetry.pin('footpadAdc1V', 0.1);
    telemetry.pin('footpadAdc2V', 2.7);
    telemetry.pin('dutyCycle', 0.78);
    const speech: { at: number; value: SpeechEffect }[] = [];
    const target = makeDirectTarget({ telemetry, onSpeech: (value) => speech.push({ at: performance.now(), value }) });
    let client: ConformanceClient | undefined;
    try {
      const endpoint = await target.start();
      client = await ConformanceClient.open(endpoint.url, endpoint.origin);
      let state = await client.request('state.get', {});
      assert.equal(state.settings.announcements.footpadEnabled, false);
      assert.equal(state.settings.announcements.footpadDelaySeconds, 1);
      await client.request('settings.update', {
        expectedRevision: state.settings.revision,
        patch: {
          safetyDisclaimerAccepted: true,
          announcements: {
            ...state.settings.announcements,
            footpadEnabled: true,
            footpadDelaySeconds: 2,
            repeatIntervalSeconds: 1,
          },
        },
      });
      await client.request('bluetooth.requestPermission', {});
      const connect = async () => {
        await client!.request('bluetooth.scan', { timeoutMs: 10000 });
        const scanned = await client!.waitForState((s) => s.discoveredDevices.length > 0);
        await client!.request('device.connect', {
          deviceId: scanned.discoveredDevices[0]!.deviceId,
          trackingMode: 'guest',
        });
        await client!.waitForState(
          (s) => s.activeConnection?.state === 'ready' && s.activeConnection.latestSample?.footpadState === 'right'
        );
      };
      await connect();
      const waitSpeech = async (count: number) => {
        const deadline = performance.now() + 7000;
        while (speech.length < count && performance.now() < deadline) await pause(25);
        assert.equal(speech.length, count);
        return speech[count - 1]!;
      };
      const first = await waitSpeech(1);
      assert.equal(first.value.footpad, 'left');
      assert.equal(first.value.load, null);
      assert.equal(first.value.speed, null);
      const repeat = await waitSpeech(2);
      assert.ok(repeat.at - first.at >= 990);
      state = await client.request('state.get', {});
      await client.request('settings.update', {
        expectedRevision: state.settings.revision,
        patch: {
          announcements: {
            ...state.settings.announcements,
            speedEnabled: true,
            loadEnabled: true,
          },
        },
      });
      const combined = await waitSpeech(3);
      assert.equal(combined.value.footpad, 'left');
      assert.equal(combined.value.speed, 32);
      assert.deepEqual(combined.value.load, { source: 'duty', percent: 78 });
      telemetry.pin('footpadState', 'both');
      state = await client.request('state.get', {});
      await client.request('settings.update', {
        expectedRevision: state.settings.revision,
        patch: {
          announcements: {
            ...state.settings.announcements,
            speedEnabled: false,
            loadEnabled: false,
          },
        },
      });
      await client.waitForState((s) => s.activeConnection?.latestSample?.footpadState === 'both');
      const recovered = speech.length;
      await pause(1200);
      assert.equal(speech.length, recovered);
      telemetry.pin('footpadState', 'right');
      await client.waitForState((s) => s.activeConnection?.latestSample?.footpadState === 'right');
      await pause(700);
      await client.request('device.disconnect', {});
      await connect();
      const reconnected = performance.now();
      const before = speech.length;
      await pause(1000);
      assert.equal(speech.length, before);
      const after = await waitSpeech(before + 1);
      assert.ok(after.at - reconnected >= 1500);
      assert.equal(after.value.footpad, 'left');
    } finally {
      client?.close();
      await target.stop();
    }
  }
);

for (const perCell of [false, true])
  test(
    `direct: battery charge warnings use ${perCell ? 'per-cell' : 'pack'} limits and running state without a speed gate`,
    { timeout: 35_000 },
    async () => {
      const telemetry = new InteractiveTelemetrySource();
      telemetry.setScene('idle');
      telemetry.pin('packageState', 'ready');
      telemetry.pin('rideState', 'idle');
      telemetry.pin('speedKph', 0);
      telemetry.pin('packVoltageV', 75);
      const speech: SpeechEffect[] = [];
      const target = makeDirectTarget({
        telemetry,
        refloatVersion: perCell ? '1.2.3' : '1.0.0',
        onSpeech: (value) => speech.push(value),
      });
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
        await client.request('bluetooth.requestPermission', {});
        const connect = async () => {
          await client!.request('bluetooth.scan', { timeoutMs: 10000 });
          const scanned = await client!.waitForState((s) => s.discoveredDevices.length > 0);
          await client!.request('device.connect', {
            deviceId: scanned.discoveredDevices[0]!.deviceId,
            trackingMode: 'saved',
          });
          await client!.waitForState((s) => s.activeConnection?.state === 'ready');
        };
        await connect();
        await client.waitForState((s) => s.activeConnection?.refloatConfiguration?.currentLoaded === true, {
          timeoutMs: 15_000,
        });
        const ready = await client.request('state.get', {});
        const savedDeviceId = ready.activeConnection!.savedDeviceId!;
        const profile = await client.request('boardProfile.get', { savedDeviceId });
        await client.request('boardProfile.update', {
          savedDeviceId,
          expectedRevision: profile.revision,
          patch: { seriesCount: 20, parallelCount: 2, batteryCellProfileId: 'molicel-p45b' },
        });
        const current = await client.request('refloat.configuration.read', { source: 'current', refresh: false });
        await client.request('refloat.configuration.write', {
          packageVersion: current.identity.packageVersion,
          schemaFingerprint: current.identity.schemaFingerprint,
          wireSignature: current.identity.wireSignature,
          baseRevision: current.revision,
          values: {
            ...current.values,
            tiltback_lv: { type: 'double', value: perCell ? 3 : 60 },
            tiltback_hv: { type: 'double', value: perCell ? 4.3 : 86 },
          },
        });
        const waitBattery = async (kind: 'low' | 'overVoltage') => {
          const deadline = performance.now() + 7000;
          while (!speech.some((s) => s.battery === kind) && performance.now() < deadline) await pause(25);
          const value = speech.find((s) => s.battery === kind);
          assert.ok(value, `Expected ${kind} announcement`);
          assert.equal(value.footpad, null);
          assert.equal(value.load, null);
          assert.equal(value.speed, null);
        };
        telemetry.pin('packVoltageV', 66.4);
        await client.waitForState((s) => s.activeConnection?.batteryWarning === 'low', { timeoutMs: 10_000 });
        assert.equal(speech.length, 0, 'earlier stationary low visual does not trigger speech');
        telemetry.pin('packVoltageV', 62);
        await pause(2300);
        assert.equal(speech.length, 0, 'non-running state suppresses low warning');
        telemetry.pin('packageState', 'running');
        telemetry.pin('rideState', 'riding');
        await waitBattery('low');
        await client.waitForState((s) => s.activeConnection?.batteryWarning === 'low', { timeoutMs: 10_000 });
        telemetry.pin('packageState', 'ready');
        telemetry.pin('rideState', 'idle');
        telemetry.pin('packVoltageV', 85);
        telemetry.pin('batteryCurrentA', -5);
        await client.waitForState((s) => s.activeConnection?.batteryWarning === 'overVoltage');
        assert.equal(speech.length, 1, 'stationary above-full visual does not trigger speech');
        telemetry.pin('packageState', 'running');
        telemetry.pin('rideState', 'riding');
        telemetry.pin('batteryCurrentA', 5);
        await pause(1000);
        assert.equal(speech.length, 1, 'advance high warning requires regen');
        telemetry.pin('batteryCurrentA', -5);
        await waitBattery('overVoltage');
        await client.waitForState((s) => s.activeConnection?.batteryWarning === 'overVoltage');
        await pause(1200);
        assert.equal(speech.length, 2, 'battery repeats do not use routine five-second interval');
        state = await client.request('state.get', {});
        await client.request('settings.update', {
          expectedRevision: state.settings.revision,
          patch: {
            announcements: { ...state.settings.announcements, batteryChargeEnabled: false },
          },
        });
        telemetry.pin('packVoltageV', 59);
        await client.request('device.disconnect', {});
        await connect();
        assert.equal((await client.request('state.get', {})).settings.announcements.batteryChargeEnabled, false);
        await pause(2300);
        assert.equal(speech.length, 2, 'explicit off persists through reconnect');
        await client.waitForState((s) => s.activeConnection?.batteryWarning === 'low', { timeoutMs: 10_000 });
        telemetry.pin('packageState', 'ready');
        await pause(300);
        assert.equal((await client.request('state.get', {})).activeConnection?.batteryWarning, 'low');
        telemetry.pin('packVoltageV', 75);
        await client.waitForState((s) => s.activeConnection?.batteryWarning === null, { timeoutMs: 10_000 });
      } finally {
        client?.close();
        await target.stop();
      }
    }
  );
