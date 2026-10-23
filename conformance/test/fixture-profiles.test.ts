import assert from 'node:assert/strict';
import test from 'node:test';
import { composeBoard, expressThor } from '../fixture/boards.ts';
import { encodeFirmwarePayload, firmwareVariants, thor300, type FirmwareVersion } from '../fixture/firmware.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { encodeRefloatTelemetry, refloatCodecs, type RefloatVersion } from '../fixture/refloat.ts';
import { idleTelemetry, InteractiveTelemetrySource, type TelemetrySample } from '../fixture/telemetry.ts';
import type { FixtureTopology } from '../fixture/topology.ts';
import { VescSimulator } from '../fixture/simulator.ts';

const firmwareVersions = Object.keys(firmwareVariants) as FirmwareVersion[];
const refloatVersions = Object.keys(refloatCodecs) as RefloatVersion[];
const topologies: FixtureTopology[] = ['direct', 'express'];

test('stable INFO reproduces the upstream patch-as-suffix build artifact', () => {
  for (const version of refloatVersions.filter((version) => !version.startsWith('1.0.'))) {
    const payload = refloatCodecs[version].infoPayload;
    const suffix = Buffer.from(payload.slice(28, 48)).toString('ascii').replace(/\0+$/, '');
    assert.equal(suffix, version.split('.')[2], version);
  }
});

test('firmware variants retain THOR300 identity and version-specific tail shape', () => {
  assert.equal(firmwareVariants['6.05'].payload.length, firmwareVariants['6.02'].payload.length + 4);
  assert.equal(firmwareVariants['6.06'].payload.length, firmwareVariants['6.05'].payload.length);
  assert.equal(firmwareVariants['7.00'].payload.length, firmwareVariants['6.06'].payload.length);
  for (const version of firmwareVersions) {
    const payload = firmwareVariants[version].payload;
    const [major, minor] = version.split('.').map(Number);
    assert.deepEqual([...payload.slice(0, 3)], [0, major, minor]);
    assert.equal(new TextDecoder().decode(payload.slice(3, 10)), 'THOR300');
    assert.equal(payload[31], 0, 'firmware name must be explicitly terminated');
  }
});

for (const firmware of firmwareVersions) {
  for (const refloat of refloatVersions) {
    for (const topology of topologies) {
      test(`${topology} composes THOR300 ${firmware} with Refloat ${refloat}`, () => {
        const telemetry = new InteractiveTelemetrySource(7);
        telemetry.start();
        const simulator = new VescSimulator(
          composeBoard({
            firmware,
            refloat,
            topology,
            canId: 42,
            telemetry,
            requestPolicy: 'interactive',
          })
        );
        const prefix = topology === 'express' ? [34, 42] : [];
        const firmwareResponse = topology === 'express' ? single(simulator, [34, 42, 0]) : single(simulator, [0]);
        const firmwarePayload = decode(firmwareResponse.frame);
        const [major, minor] = firmware.split('.').map(Number);
        assert.deepEqual([...firmwarePayload.slice(0, 3)], [0, major, minor]);
        if (topology === 'express') {
          assert.equal(decode(single(simulator, [0]).frame)[2], 5);
          assert.deepEqual([...decode(single(simulator, [62]).frame)], [62, 42]);
        }

        const info = decode(single(simulator, [...prefix, 36, 101, 0, 2, 0]).frame);
        const legacy = refloat.startsWith('1.0.');
        assert.equal(info[3], legacy ? 10 : 2);
        if (!legacy) {
          assert.deepEqual([...info.slice(25, 28)], refloat.split('.').map(Number));
          if (refloat !== '1.3.0') {
            assert.equal(single(simulator, [...prefix, 36, 101, 32]).request.kind, 'refloat-layout');
          }
        }
        const request = legacy
          ? [...prefix, 36, 101, 10]
          : refloat === '1.3.0'
            ? [...prefix, 36, 101, 33, 1, 0x7f, 0xff, 0xbf, 0xc3, 0, 0, 0, 0]
            : [...prefix, 36, 101, 31];
        const telemetryResponse = single(simulator, request);
        assert.equal(telemetryResponse.request.telemetryPhase, 'riding');
        if (refloat === '1.3.0') {
          assert.equal(decode(telemetryResponse.frame)[17]! & 3, 3, 'command 33 must encode the running source state');
        }
        assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(6))), []);
      });
    }
  }
}

test('interactive Express forwarding ignores another CAN identity without falling through', () => {
  const simulator = new VescSimulator(
    composeBoard({
      firmware: '6.06',
      refloat: '1.2.3',
      topology: 'express',
      canId: 42,
    })
  );
  assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(34, 41, 0))), []);
  assert.match(simulator.diagnostics.at(-1) ?? '', /Unknown CAN target 41/);
});

test('multi-node CAN networks retain raw ping anomalies and route-specific identities', () => {
  const secondIdentity = {
    ...thor300,
    uuid: Uint8Array.from({ length: 12 }, (_, index) => index + 1),
  };
  const second = {
    ...expressThor.controllers[0]!,
    canId: 42,
    firmware: {
      ...expressThor.controllers[0]!.firmware,
      payload: encodeFirmwarePayload(secondIdentity, 6, 5),
    },
  };
  const omitted = {
    ...expressThor.controllers[0]!,
    canId: 99,
    firmware: {
      ...expressThor.controllers[0]!.firmware,
      payload: encodeFirmwarePayload({ ...thor300, uuid: new Uint8Array(12).fill(0x55) }, 6, 2),
    },
  };
  const simulator = new VescSimulator({
    ...expressThor,
    controllers: [
      {
        ...expressThor.controllers[0]!,
        canId: 7,
        firmware: expressThor.transport,
      },
      second,
      omitted,
    ],
    canPingIds: [42, 7, 42, 254, 255],
    silentCanIds: [88],
    requestPolicy: 'interactive',
  });

  assert.deepEqual([...decode(single(simulator, [62]).frame)], [62, 42, 7, 42, 254, 255]);
  assert.equal(decode(single(simulator, [34, 42, 0]).frame)[2], 5);
  assert.equal(decode(single(simulator, [34, 99, 0]).frame)[2], 2);
  assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(34, 88, 0))), []);
  assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(34, 77, 0))), []);
  assert.match(simulator.diagnostics.join('\n'), /Silent CAN target 88/);
  assert.match(simulator.diagnostics.join('\n'), /Unknown CAN target 77/);
});

test('simulator can emit a discovery-overflow ping without allocating controllers', () => {
  const simulator = new VescSimulator({
    ...expressThor,
    canPingIds: Array.from({ length: 65 }, (_, index) => index),
  });
  assert.equal(decode(single(simulator, [62]).frame).length, 66);
});

test('seeded telemetry is smooth, reproducible, correlated, and stops immediately', () => {
  const left = new InteractiveTelemetrySource(123);
  const right = new InteractiveTelemetrySource(123);
  assert.equal(left.start(), true);
  assert.equal(left.start(), false);
  right.start();
  const samples = Array.from({ length: 12 }, () => left.next());
  const repeated = Array.from({ length: 12 }, () => right.next());
  assert.deepEqual(samples, repeated);
  for (let index = 1; index < samples.length; index += 1) {
    assert.ok(Math.abs(samples[index]!.speedKph - samples[index - 1]!.speedKph) < 2);
    assert.ok(samples[index]!.erpm > samples[index]!.speedKph * 200);
    assert.ok(samples[index]!.packVoltageV < idleTelemetry.packVoltageV);
  }
  assert.equal(left.stop(), true);
  assert.equal(left.stop(), false);
  assert.deepEqual(left.next(), idleTelemetry);
});

test('all Refloat encoders retain equivalent normalized core state', () => {
  const sample: TelemetrySample = {
    ...idleTelemetry,
    rideState: 'riding',
    packageState: 'running',
    footpadState: 'both',
    speedKph: 21.6,
    erpm: 5000,
    dutyCycle: 0.5,
    motorCurrentA: 24,
    batteryCurrentA: 10,
    packVoltageV: 62,
    controllerTempC: 34,
    motorTempC: 32,
  };
  for (const version of refloatVersions) {
    const payload = encodeRefloatTelemetry(version, sample);
    const legacy = version.startsWith('1.0.');
    assert.equal(payload[0], 36);
    assert.equal(payload[1], 101);
    const selectable = version === '1.3.0';
    assert.equal(legacy ? payload[10]! & 0x0f : selectable ? payload[17]! & 3 : payload[9], legacy ? 1 : 3);
    assert.equal(legacy ? payload[11] : selectable ? payload[18]! >> 6 : payload[10]! >> 6, legacy ? 2 : 3);
  }
});

function single(simulator: VescSimulator, payload: number[]) {
  const responses = simulator.receive(encodeFrame(Uint8Array.from(payload)));
  assert.equal(responses.length, 1);
  return responses[0]!;
}

function decode(frame: Uint8Array) {
  const payloads = new FrameAssembler().append(frame);
  assert.equal(payloads.length, 1);
  return payloads[0]!;
}
