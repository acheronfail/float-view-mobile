/** Interactive physical announcement checkpoint; never infer audibility from TTS completion. */
import assert from 'node:assert/strict';
import { createInterface } from 'node:readline';
import { androidPhysicalTarget } from '../targets/android.ts';
import { FixtureClient } from '../fixture/serial.ts';
import { FixtureRunner } from '../fixture/runner.ts';
import { physicalScenario } from '../fixture/scenarios.ts';
import { InteractiveTelemetrySource } from '../fixture/telemetry.ts';
import { ConformanceClient } from './client.ts';
import type { AnnouncementSettings } from '../../frontend/src/lib/backend/contract.ts';

const [fixturePath, serial] = process.argv.slice(2);
assert.ok(fixturePath && serial, 'Usage: announcements-physical.ts <fixture-port> <Android serial>');
const target = androidPhysicalTarget(serial);
const scenario = physicalScenario('direct', '1.3.0');
const telemetry = scenario.board.controllers[0]!.telemetry;
assert.ok(telemetry instanceof InteractiveTelemetrySource);
telemetry.setScene('idle');
const input = createInterface({ input: process.stdin, terminal: false });
const deadline = setTimeout(() => input.close(), 20 * 60_000);
let runner: FixtureRunner | undefined;
let client: ConformanceClient | undefined;
let restoreSettings: (() => Promise<void>) | undefined;
let clean = false;
process.once('SIGINT', () => input.close());
process.once('SIGTERM', () => input.close());
try {
  await target.prepareSuite?.(console.log);
  const fixture = await FixtureClient.open(fixturePath);
  runner = new FixtureRunner(scenario, fixture, undefined, 1000, console.log);
  await runner.start();
  const endpoint = await target.start();
  client = await ConformanceClient.open(endpoint.url, endpoint.origin, 10_000);
  const saved = (await client.request('state.get', {})).settings;
  restoreSettings = async () => {
    const current = await client!.request('state.get', {});
    await client!.request('settings.update', {
      expectedRevision: current.settings.revision,
      patch: {
        announcements: saved.announcements,
        unitPreference: saved.unitPreference,
        minimumSessionDistance: saved.minimumSessionDistance,
        locationRecordingEnabled: saved.locationRecordingEnabled,
      },
    });
    const restored = (await client!.request('state.get', {})).settings;
    assert.deepEqual(restored.announcements, saved.announcements);
    assert.equal(restored.unitPreference, saved.unitPreference);
    assert.equal(restored.minimumSessionDistance, saved.minimumSessionDistance);
    assert.equal(restored.locationRecordingEnabled, saved.locationRecordingEnabled);
    console.log('RESTORE settings verified');
  };
  async function configure(patch: Partial<AnnouncementSettings>) {
    const state = await client!.request('state.get', {});
    await client!.request('settings.update', {
      expectedRevision: state.settings.revision,
      patch: { announcements: { ...state.settings.announcements, ...patch } },
    });
  }
  await configure({ speedEnabled: false, loadEnabled: false, repeatIntervalSeconds: 5, volumePercent: 100 });
  await client.request('bluetooth.requestPermission', {}, 15_000);
  await client.request('bluetooth.scan', { timeoutMs: 10_000 });
  const discovered = await client.waitForState(
    (s) => s.discoveredDevices.some((d) => d.name === scenario.board.advertisedName),
    { timeoutMs: 15_000 }
  );
  const device = discovered.discoveredDevices.find((d) => d.name === scenario.board.advertisedName)!;
  await client.request('device.connect', { deviceId: device.deviceId, trackingMode: 'saved' }, 15_000);
  await client.waitForState((s) => s.activeConnection?.state === 'ready' && !!s.activeConnection.latestSample, {
    timeoutMs: 25_000,
  });
  const configured = await client.request('state.get', {});
  await client.request('settings.update', {
    expectedRevision: configured.settings.revision,
    patch: { minimumSessionDistance: 0, locationRecordingEnabled: false, unitPreference: 'metric' },
  });
  console.log(
    'READY commands: status; test <speed|load|combined> <0..100>; phase <below|equal|duty|motor|battery|speed|combined>; volume <0..100>; off; background; foreground; reconnect; stop'
  );
  for await (const line of input) {
    assert.ok(line.length <= 128, 'Command too long');
    const [command, arg, value] = line.trim().split(/\s+/);
    if (command === 'stop') {
      clean = true;
      break;
    }
    try {
      if (command === 'test') {
        assert.ok(arg === 'speed' || arg === 'load' || arg === 'combined');
        const volume = Number(value);
        assert.ok(Number.isInteger(volume) && volume >= 0 && volume <= 100);
        await configure({ speedEnabled: false, loadEnabled: false, volumePercent: volume });
        console.log('TEST', arg, volume, await client.request('announcements.test', { kind: arg }, 12_000));
      } else if (command === 'phase') {
        assert.ok(['below', 'equal', 'duty', 'motor', 'battery', 'speed', 'combined'].includes(arg!));
        const limits = (await client.request('state.get', {})).activeConnection!.currentLimits!;
        assert.ok(limits);
        telemetry.setScene('cruise');
        const motor = arg === 'motor' ? limits.motorCurrentMaxA * 0.78 : 0;
        const battery = arg === 'battery' ? limits.batteryCurrentMaxA * 0.82 : 0;
        for (const field of ['motorCurrentA', 'directionalMotorCurrentA', 'filteredMotorCurrentA'] as const)
          telemetry.pin(field, motor);
        telemetry.pin('batteryCurrentA', battery);
        telemetry.pin(
          'dutyCycle',
          arg === 'below'
            ? 0.64
            : arg === 'equal'
              ? 0.65
              : arg === 'speed'
                ? 0.2
                : arg === 'motor' || arg === 'battery'
                  ? 0.68
                  : 0.78
        );
        telemetry.pin(
          'speedKph',
          arg === 'below' ? 24 : arg === 'equal' ? 25 : arg === 'combined' || arg === 'speed' ? 32 : 0
        );
        await configure({
          speedEnabled: true,
          loadEnabled: true,
          speedThresholdKph: 25,
          loadThresholdPercent: 65,
          repeatIntervalSeconds: 5,
          volumePercent: 100,
        });
        const expectedSpeed =
          arg === 'below' ? 24 : arg === 'equal' ? 25 : arg === 'combined' || arg === 'speed' ? 32 : 0;
        const expectedSource = arg === 'motor' ? 'motor' : arg === 'battery' ? 'battery' : 'duty';
        await client.waitForState(
          (s) => {
            const sample = s.activeConnection?.latestSample;
            return (
              !!sample &&
              Math.abs((sample.speedKph ?? NaN) - expectedSpeed) < 0.05 &&
              sample.load.winner?.source === expectedSource
            );
          },
          { timeoutMs: 10_000 }
        );
        console.log('PHASE', arg, 'configured; confirm audible behavior separately');
      } else if (command === 'off') {
        await configure({ speedEnabled: false, loadEnabled: false });
        console.log('OFF automatic rules disabled');
      } else if (command === 'volume') {
        const volume = Number(arg);
        assert.ok(Number.isInteger(volume) && volume >= 0 && volume <= 100);
        await configure({ volumePercent: volume });
        console.log('VOLUME', volume);
      } else if (command === 'background') {
        assert.ok(target.background);
        await target.background();
        console.log('BACKGROUND');
      } else if (command === 'foreground') {
        assert.ok(target.foreground);
        await target.foreground();
        console.log('FOREGROUND');
      } else if (command === 'reconnect') {
        await runner.injectDisconnectAndReadvertise();
        console.log('RECONNECT injected');
      } else if (command !== 'status') {
        throw new Error('Unknown command');
      }
      const state = await client.request('state.get', {});
      const c = state.activeConnection;
      console.log(
        'STATUS',
        JSON.stringify({
          audio: state.announcementAudio,
          announcements: state.settings.announcements,
          connection: c?.state,
          sequence: c?.latestSample?.sequence,
          speed: c?.latestSample?.speedKph,
          load: c?.latestSample?.load,
          sessionSamples: c?.activeSession?.sampleCount,
        })
      );
    } catch (error) {
      console.log('COMMAND FAILED', error instanceof Error ? error.message : String(error));
    }
  }
} finally {
  clearTimeout(deadline);
  input.close();
  let failure: unknown;
  try {
    if (client) await client.request('device.disconnect', {}, 15_000);
  } catch (error) {
    failure = error;
  }
  try {
    await restoreSettings?.();
  } catch (error) {
    failure ??= error;
  }
  client?.close();
  try {
    if (runner) await runner.stop(clean && !failure);
  } catch (error) {
    failure ??= error;
  }
  await target.finishSuite?.(console.log);
  if (failure) throw failure;
  console.log('CLEANUP fixture reset, settings restored, app stopped and tunnel removed');
}
