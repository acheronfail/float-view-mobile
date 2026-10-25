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
