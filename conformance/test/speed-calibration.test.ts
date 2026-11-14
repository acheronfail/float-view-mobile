import assert from 'node:assert/strict';
import { test } from 'node:test';
import { makeDirectTarget } from '../targets/direct.ts';
import { InteractiveTelemetrySource } from '../fixture/telemetry.ts';
import { ConformanceClient } from '../src/client.ts';
import { assertState } from '../src/suite.ts';

test('direct: temporary GPS calibration recommends and applies only the diameter', { timeout: 150_000 }, async () => {
  const telemetry = new InteractiveTelemetrySource();
  const target = makeDirectTarget({
    telemetry,
    calibrationSpeedMps: 4.8,
    calibrationAccuracyMps: (observation) => (observation <= 8 ? 0.05 : 0.9),
    calibrationAgeMs: 600,
    refloatVersion: '1.3.1',
    onSpeech: () => {},
  });
  let client: ConformanceClient | undefined;
  try {
    const endpoint = await target.start();
    client = await ConformanceClient.open(endpoint.url, endpoint.origin);
    let state = await client.request('state.get', {});
    await client.request('settings.update', {
      expectedRevision: state.settings.revision,
      patch: { safetyDisclaimerAccepted: true, minimumSessionDistance: 0 },
    });
    await client.request('bluetooth.scan', { timeoutMs: 10_000 });
    state = await client.waitForState((s) => s.discoveredDevices.length > 0);
    await client.request('device.connect', { deviceId: state.discoveredDevices[0]!.deviceId, trackingMode: 'saved' });
    state = await client.waitForState(
      (s) => s.activeConnection?.state === 'ready' && s.speedCalibration.currentDiameterMm !== null
    );
    assertState(state);
    assert.equal(state.settings.locationRecordingEnabled, false);
    await client.request('speedCalibration.start', { diameterMm: 280 });
    state = await client.waitForState((s) => s.speedCalibration.measurementStatus === 'waitingGps');
    telemetry.setScene('cruise');
    telemetry.pin('speedKph', 18);
    telemetry.pin('wheelslip', false);
    state = await client.waitForState((s) => s.speedCalibration.acceptedSamples >= 32, { timeoutMs: 45_000 });
    assert.equal(state.speedCalibration.status, 'collecting');
    assert.equal(state.speedCalibration.measurementStatus, 'needsMoreMeasurements');
    assert.equal(state.speedCalibration.recommendedDiameterMm, null);
    assert.ok(state.speedCalibration.progressPercent > 0 && state.speedCalibration.progressPercent < 100);
    state = await client.waitForState((s) => s.speedCalibration.status === 'refining', { timeoutMs: 45_000 });
    assert.ok(state.speedCalibration.acceptedSamples > 32);
    assert.equal(state.speedCalibration.recommendedDiameterMm, 268.8);
    assert.equal(state.speedCalibration.progressPercent, 100);
    assert.equal(state.speedCalibration.measurementStatus, 'ready');
    assert.equal(state.speedCalibration.status, 'refining');
    await assert.rejects(client.request('speedCalibration.apply', {}));
    const firstReadyCount = state.speedCalibration.acceptedSamples;
    state = await client.waitForState(
      (s) => s.speedCalibration.status === 'refining' && s.speedCalibration.acceptedSamples >= firstReadyCount + 3,
      { timeoutMs: 10_000 }
    );
    assert.equal(state.speedCalibration.applyAvailable, false);
    await client.request('speedCalibration.finish', {});
    state = await client.waitForState((s) => s.speedCalibration.status === 'ready');
    await assert.rejects(client.request('speedCalibration.apply', {}));
    telemetry.stop();
    telemetry.unpin('speedKph');
    state = await client.waitForState((s) => s.speedCalibration.applyAvailable, { timeoutMs: 5_000 });
    assert.equal(state.activeConnection?.latestLocation, null);
    await client.request('speedCalibration.apply', {}, 12_000);
    state = await client.waitForState((s) => s.speedCalibration.status === 'applied');
    assert.ok(Math.abs(state.speedCalibration.currentDiameterMm! - 268.8) < 0.001);
    assert.equal(state.settings.locationRecordingEnabled, false);
    assertState(state);
  } finally {
    await client?.close();
    await target.stop();
  }
});
