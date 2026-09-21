import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { composeBoard } from '../fixture/boards.ts';
import { encodeFrame } from '../fixture/frame.ts';
import {
  encodeDynamicTelemetry,
  encodeLegacyCompact,
  encodeLegacyRealtime,
  encodeRefloatLayout,
  encodeSelectableTelemetry,
  MAX_REFLOAT_FIELD_BYTES,
  realtimeFields,
  refloatCodecs,
  runtimeRealtimeFields,
} from '../fixture/refloat.ts';
import { chunks } from '../fixture/runner.ts';
import { VescSimulator } from '../fixture/simulator.ts';
import {
  recentBounded,
  TELEMETRY_LIMITS,
  validateExtensions,
  validateJson,
  validateMessage,
  validateScenarioDocument,
} from '../fixture/telemetry-contract.ts';
import { InteractiveTelemetrySource, telemetryScenes } from '../fixture/telemetry.ts';

const sourceFixture = JSON.parse(
  readFileSync(new URL('../../backends/fixtures/refloat/detailed-source-provenance.json', import.meta.url), 'utf8')
) as {
  cases: Array<{ name: string; payloadHex: string }>;
  deferredCommands: number[];
};

function expected(name: string) {
  const value = sourceFixture.cases.find((item) => item.name === name);
  assert.ok(value, `Missing source fixture ${name}`);
  return value.payloadHex;
}

function hex(value: Uint8Array) {
  return Buffer.from(value).toString('hex');
}

test('immutable source fixtures match command 1 and command 10 detail levels 1 through 4', () => {
  const source = new InteractiveTelemetrySource(7);
  source.setScene('acceleration');
  const sample = source.next();
  assert.equal(hex(encodeLegacyRealtime(sample)), expected('1.0.0-command-1-high-precision'));
  for (const level of [1, 2, 3, 4]) {
    assert.equal(hex(encodeLegacyCompact(sample, level)), expected(`1.0.0-command-10-detail-${level}`));
  }
  assert.equal(encodeLegacyRealtime(sample).length, 73);
  assert.deepEqual(
    [1, 2, 3, 4].map((level) => encodeLegacyCompact(sample, level).length),
    [35, 42, 55, 59]
  );
});

test('dynamic source fixtures retain all 16 always-present and 10 running-only fields', () => {
  const layout = encodeRefloatLayout();
  assert.equal(hex(layout), expected('1.1.0-and-1.2.3-command-32-layout'));
  assert.equal(realtimeFields.length, 16);
  assert.equal(runtimeRealtimeFields.length, 10);

  const source = new InteractiveTelemetrySource(7);
  source.setScene('acceleration');
  const sample = source.next();
  assert.equal(hex(encodeDynamicTelemetry(sample, false)), expected('1.1.0-command-31-running'));
  assert.equal(hex(encodeDynamicTelemetry(sample, true)), expected('1.2.3-command-31-running-alert-tail'));
});

test('charging and active firmware fault tails are source-faithful and distinct', () => {
  const charging = new InteractiveTelemetrySource();
  charging.setScene('charging');
  assert.equal(hex(encodeDynamicTelemetry(charging.next(), true)), expected('1.2.3-command-31-charging-tail'));
  const fault = new InteractiveTelemetrySource();
  fault.setScene('firmwareFault');
  const sample = fault.next();
  assert.equal(hex(encodeDynamicTelemetry(sample, true)), expected('1.2.3-command-31-active-firmware-fault'));
  assert.equal(hex(encodeLegacyCompact(sample, 4)), expected('1.2.3-command-10-active-firmware-fault'));
});

test('Refloat 1.3.0 serves fixed command-33 masks and keeps command 10 fault-only', () => {
  const source = new InteractiveTelemetrySource(7);
  source.setScene('acceleration');
  const sample = source.next();
  for (const [name, requestHex] of [
    ['1.3.0-command-33-fast-float32', '246521017fffbfc300000000'],
    ['1.3.0-command-33-slow-gnss-float32', '246521010000400000007fff'],
  ] as const) {
    const request = Uint8Array.from(Buffer.from(requestHex, 'hex'));
    assert.equal(hex(encodeSelectableTelemetry(request, sample)), expected(name));
  }
  const response = refloatCodecs['1.3.0'].respond(Uint8Array.of(36, 101, 10, 0), sample);
  assert.equal(hex(response!.payload), expected('1.3.0-command-10-mode-0-normal'));
  assert.equal(refloatCodecs['1.3.0'].layoutPayload, undefined);
});

test('layout, extension, JSON, message, diagnostic, point, and event bounds are concrete', () => {
  const exactNames = Array.from({ length: 64 }, (_, index) => `${String(index).padStart(2, '0')}${'x'.repeat(62)}`);
  assert.equal(
    exactNames.every((value) => Buffer.byteLength(value) === 64),
    true
  );
  assert.doesNotThrow(() => encodeRefloatLayout(exactNames, []));
  assert.throws(() => encodeRefloatLayout([...exactNames, 'overflow'], []), /64 fields/);
  assert.throws(() => encodeRefloatLayout(['x'.repeat(MAX_REFLOAT_FIELD_BYTES + 1)], []), /UTF-8 bound/);
  assert.throws(
    () =>
      encodeRefloatLayout(
        exactNames.slice(0, 64).map((value) => `${value}y`),
        []
      ),
    /UTF-8 bound/
  );

  const extensions = Object.fromEntries(
    Array.from({ length: TELEMETRY_LIMITS.extensionFields }, (_, index) => [`future.${index}`, index])
  );
  assert.doesNotThrow(() => validateExtensions(extensions));
  assert.throws(() => validateExtensions({ ...extensions, overflow: 33 }), /32 entries/);
  assert.throws(() => validateExtensions({ bad: Number.NaN }), /finite/);
  assert.doesNotThrow(() => validateJson({ value: 'x'.repeat(16_360) }));
  assert.throws(() => validateJson({ value: 'x'.repeat(16_384) }), /UTF-8 bound/);
  assert.doesNotThrow(() => validateMessage(new Uint8Array(65_536)));
  assert.throws(() => validateMessage(new Uint8Array(65_537)), /65,536/);
  assert.doesNotThrow(() => validateScenarioDocument('x'.repeat(65_536)));
  assert.throws(() => validateScenarioDocument('x'.repeat(65_537)), /65,536/);
  assert.equal(recentBounded(Array.from({ length: 51 }), 50).values.length, 50);
  assert.equal(recentBounded(Array.from({ length: 2_001 }), 2_000).truncated, true);
});

test('semantic scenes, pinning, freeze, resume, reset, and finite validation are deterministic', () => {
  assert.ok(telemetryScenes.includes('fieldWeakening'));
  assert.ok(telemetryScenes.includes('firmwareFault'));
  assert.ok(telemetryScenes.includes('speedSweep'));
  const left = new InteractiveTelemetrySource(42);
  const right = new InteractiveTelemetrySource(42);
  left.setScene('acceleration');
  right.setScene('acceleration');
  assert.deepEqual(left.next(), right.next());
  left.pin('controllerTempC', 77);
  assert.equal(left.next().controllerTempC, 77);
  assert.equal(left.freeze(), true);
  const frozen = left.next();
  assert.deepEqual(left.next(), frozen);
  assert.equal(left.resume(), true);
  assert.notDeepEqual(left.next(), frozen);
  left.reset();
  assert.deepEqual(left.pinnedFields(), {});
  assert.throws(() => left.pin('motorCurrentA', Number.POSITIVE_INFINITY), /finite/);
  assert.throws(() => left.pin('remoteInput', 2), /\[-1, 1\]/);
});

test('speed sweep scene makes large coherent changes for gauge animation checks', () => {
  const source = new InteractiveTelemetrySource();
  source.setScene('speedSweep');
  const samples = Array.from({ length: 48 }, () => source.next());
  const speeds = samples.map((sample) => sample.speedKph);
  assert.ok(Math.max(...speeds) - Math.min(...speeds) > 30);
  assert.equal(
    samples.every((sample) => sample.erpm === Math.round(sample.speedKph * 235)),
    true
  );
});

test('simulator routes realtime and compact requests and bounds unsupported diagnostics', () => {
  const telemetry = new InteractiveTelemetrySource();
  telemetry.setScene('fieldWeakening');
  const simulator = new VescSimulator(
    composeBoard({
      firmware: '6.06',
      refloat: '1.2.3',
      topology: 'express',
      canId: 42,
      telemetry,
      requestPolicy: 'interactive',
    })
  );
  const realtime = simulator.receive(encodeFrame(Uint8Array.of(34, 42, 36, 101, 31)));
  const compact = simulator.receive(encodeFrame(Uint8Array.of(34, 42, 36, 101, 10, 4)));
  assert.equal(realtime[0]?.request.canId, 42);
  assert.equal(compact[0]?.request.canId, 42);
  assert.equal(compact[0]?.request.kind, 'telemetry');
  assert.equal(compact[0]?.request.command, 36);
  assert.equal(realtime[0]?.request.telemetryCommand, 'realtime');
  assert.equal(compact[0]?.request.telemetryCommand, 'compact');
  for (let index = 0; index < 60; index += 1) {
    for (const command of sourceFixture.deferredCommands) {
      simulator.receive(encodeFrame(Uint8Array.of(34, 42, 36, 101, command)));
    }
  }
  assert.equal(simulator.diagnostics.length, TELEMETRY_LIMITS.diagnostics);
  assert.match(simulator.diagnostics[0] ?? '', /Unsupported Refloat request/);
});

test('response fragmentation preserves exact ordered bytes', () => {
  const response = refloatCodecs['1.2.3'].layoutPayload!;
  const fragments = chunks(response, 20);
  assert.equal(
    fragments.every((value) => value.length <= 20),
    true
  );
  assert.deepEqual(Buffer.concat(fragments.map((value) => Buffer.from(value))), Buffer.from(response));
});
