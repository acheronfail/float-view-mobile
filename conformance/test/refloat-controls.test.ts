import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  decodeRefloatProductControl,
  RefloatControlModel,
  type RefloatControlRejection,
  type RefloatProductControl,
} from '../fixture/refloat-controls.ts';
import type { RefloatVersion } from '../fixture/refloat.ts';
import { composeBoard, composeInteractiveBoard } from '../fixture/boards.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { VescSimulator } from '../fixture/simulator.ts';
import { idleTelemetry } from '../fixture/telemetry.ts';

test('Refloat history returns transient faults after the active fault clears', () => {
  let ticks = 0;
  const model = new RefloatControlModel('1.3.0', {}, () => ++ticks);
  model.recordFirmwareFault(4);
  model.recordFirmwareFault(0);
  const response = model.alertsResponse(Uint8Array.of(36, 101, 35, 0, 0, 0, 0))!;
  assert.equal(response[11], 0, 'fault is no longer active');
  assert.equal(response[12], 2, 'both start and end survive');
  assert.deepEqual([...response.slice(13)], [0, 0, 0, 1, 1, 1, 4, 0, 0, 0, 0, 2, 1, 0, 0]);
  assert.equal(model.state.fatalErrorLatched, true);
  assert.equal(new RefloatControlModel('1.1.2').alertsResponse(Uint8Array.of(36, 101, 35, 0, 0, 0, 0)), undefined);
});

test('manual workbench starts clean and retains only faults it generated', () => {
  const board = composeInteractiveBoard({
    advertisedName: 'Fault test',
    topology: 'direct',
    canPingIds: [],
    controllers: [{ alias: 'board', canId: 0, firmware: '6.06', refloat: '1.3.0', uuid: new Uint8Array(12) }],
    telemetry: { phase: () => 'idle', next: () => idleTelemetry },
  });
  const model = board.controllers[0]!.refloatControls!;
  assert.equal(model.state.fatalErrorLatched, false);
  model.applyToSample({ ...idleTelemetry, firmwareFault: 'overVoltage' });
  assert.equal(model.state.fatalErrorLatched, true);
  assert.equal(model.alertsResponse(Uint8Array.of(36, 101, 35, 0, 0, 0, 0))![11], 1);
  model.applyToSample(idleTelemetry);
  assert.equal(model.state.fatalErrorLatched, true);
  assert.equal(model.alertsResponse(Uint8Array.of(36, 101, 35, 0, 0, 0, 0))![11], 0);
  assert.equal(model.apply(Uint8Array.of(36, 101, 36, 1), { rideState: 'idle' }).accepted, true);
  assert.equal(model.state.fatalErrorLatched, false);
});

interface GoldenCase {
  name: string;
  version: RefloatVersion;
  request: number[];
  response: number[] | null;
  control: RefloatProductControl;
  enabled?: boolean;
}

interface RejectionCase {
  name: string;
  version: RefloatVersion;
  request: number[];
  selectedCanId?: number;
  requestCanId?: number;
  firmwareTestVersion?: number;
  reason: RefloatControlRejection;
}

const path = fileURLToPath(new URL('../../backends/fixtures/refloat/product-controls.json', import.meta.url));
const fixture = JSON.parse(readFileSync(path, 'utf8')) as {
  cases: GoldenCase[];
  rejections: RejectionCase[];
};

const decode = (frame: Uint8Array): Uint8Array => new FrameAssembler().append(frame)[0]!;

test('source-derived product-control golden requests match the closed decoder', () => {
  for (const item of fixture.cases) {
    const result = decodeRefloatProductControl(item.version, Uint8Array.from(item.request));
    assert.equal(result.accepted, true, item.name);
    if (!result.accepted) continue;
    assert.equal(result.value.control, item.control, item.name);
    assert.equal(result.value.enabled, item.enabled, item.name);
    if (item.response !== null) {
      assert.deepEqual(item.response.slice(0, 3), [36, 101, item.request[2]], item.name);
      assert.equal(item.response.length, 4, item.name);
    }
  }
});

test('legacy and stable zero-mask lighting requests query without changing state', () => {
  for (const [version, request, command] of [
    ['1.0.0', Uint8Array.of(36, 101, 202, 0, 1), 202],
    ['1.2.3', Uint8Array.of(36, 101, 20, 0, 0, 0, 0, 1), 20],
  ] as const) {
    const decoded = decodeRefloatProductControl(version, request);
    assert.deepEqual(decoded, {
      accepted: true,
      value: { control: 'lightingQuery', lightingMask: 0, lightingFlags: 1 },
    });

    const model = new RefloatControlModel(version, { ledsEnabled: false, headlightsEnabled: true });
    const before = { ...model.state };
    assert.deepEqual(model.apply(request, { rideState: 'riding' }), {
      accepted: true,
      response: Uint8Array.of(36, 101, command, 2),
    });
    assert.deepEqual(model.state, before, `${version} query changed disposable state`);
  }
});

test('malformed, oversized, wrong-route, and unsupported requests fail closed', () => {
  for (const item of fixture.rejections) {
    const result = decodeRefloatProductControl(item.version, Uint8Array.from(item.request), {
      selectedCanId: item.selectedCanId,
      requestCanId: item.requestCanId,
      firmwareTestVersion: item.firmwareTestVersion,
    });
    assert.deepEqual(result, { accepted: false, reason: item.reason }, item.name);
  }
});

test('direct and CAN simulators apply exact lighting responses and later one-way telemetry confirmation', () => {
  for (const [topology, prefix] of [
    ['direct', []],
    ['express', [34, 42]],
  ] as const) {
    const simulator = new VescSimulator(composeBoard({ firmware: '6.06', refloat: '1.2.3', topology, canId: 42 }));
    const lighting = simulator.receive(encodeFrame(Uint8Array.of(...prefix, 36, 101, 20, 0, 0, 0, 2, 1)));
    assert.equal(lighting.length, 1);
    assert.deepEqual([...decode(lighting[0]!.frame)], [36, 101, 20, 1]);

    assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(...prefix, 36, 101, 12, 1))), []);
    const telemetry = simulator.receive(encodeFrame(Uint8Array.of(...prefix, 36, 101, 31)));
    assert.equal(decode(telemetry[0]!.frame)[9]! & 0x03, 0, 'package disable confirms in telemetry');
    assert.equal(simulator.requests.filter((item) => item.kind === 'refloat-control').length, 2);
    assert.equal(
      simulator.requests.some((item) => item.kind === 'motor-configuration'),
      false
    );
  }
});

test('legacy lighting is exact and flywheel mode rejects every mutation except disable', () => {
  const legacy = new VescSimulator(composeBoard({ firmware: '6.02', refloat: '1.0.0', topology: 'direct' }));
  const response = legacy.receive(encodeFrame(Uint8Array.of(36, 101, 202, 1, 2)));
  assert.deepEqual([...decode(response[0]!.frame)], [36, 101, 202, 2]);

  const simulator = new VescSimulator(
    composeBoard({
      firmware: '6.06',
      refloat: '1.2.3',
      topology: 'direct',
      requestPolicy: 'interactive',
    })
  );
  assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(36, 101, 22, 0x81, 0, 0, 0, 0, 1))), []);
  for (const mutation of [
    [36, 101, 12, 1],
    [36, 101, 20, 0, 0, 0, 1, 0],
    [36, 101, 36, 1],
    [36, 101, 2, 0],
  ]) {
    assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.from(mutation))), []);
  }
  assert.match(simulator.diagnostics.join('\n'), /Rejected Refloat control|Unsupported Refloat request/);
  assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(36, 101, 22, 0x80, 0, 0, 0, 0, 1))), []);
  assert.equal(simulator.requests.filter((item) => item.kind === 'refloat-control').length, 2);
});

test('direct and CAN flywheel mode permits lighting queries but rejects lighting mutations', () => {
  for (const [refloat, topology, command] of [
    ['1.0.0', 'direct', 202],
    ['1.0.0', 'express', 202],
    ['1.2.3', 'direct', 20],
    ['1.2.3', 'express', 20],
  ] as const) {
    const prefix = topology === 'express' ? [34, 42] : [];
    const simulator = new VescSimulator(
      composeBoard({
        firmware: refloat === '1.0.0' ? '6.02' : '6.06',
        refloat,
        topology,
        canId: 42,
        requestPolicy: 'interactive',
      })
    );
    const send = (payload: number[]) => simulator.receive(encodeFrame(Uint8Array.of(...prefix, ...payload)));

    assert.deepEqual(send([36, 101, 22, 0x81, 0, 0, 0, 0, 1]), []);
    const query = command === 202 ? [36, 101, 202, 0, 3] : [36, 101, 20, 0, 0, 0, 0, 3];
    const queryResponses = send(query);
    assert.deepEqual([...decode(queryResponses[0]!.frame)], [36, 101, command, 3]);

    const mutation = command === 202 ? [36, 101, 202, 1, 2] : [36, 101, 20, 0, 0, 0, 1, 2];
    assert.deepEqual(send(mutation), []);
    assert.match(simulator.diagnostics.at(-1) ?? '', /Rejected Refloat control invalid_state/);

    assert.deepEqual(send([36, 101, 22, 0x80, 0, 0, 0, 0, 1]), []);
    assert.equal(
      simulator.requests.filter((item) => item.kind === 'refloat-control').length,
      3,
      `${refloat}-${topology}`
    );
  }
});

test('active flywheel telemetry is riding while exact disable remains authorized', () => {
  const model = new RefloatControlModel('1.2.3', { flywheelEnabled: true });
  const sample = model.applyToSample(idleTelemetry);
  assert.equal(sample.packageState, 'running');
  assert.equal(sample.packageMode, 'flywheel');
  assert.deepEqual(model.apply(Uint8Array.of(36, 101, 22, 0x80, 0, 0, 0, 0, 1), { rideState: 'riding' }), {
    accepted: true,
    oneWay: true,
  });
});

test('audited test-version simulator firmware permits controls on direct and CAN routes', () => {
  for (const topology of ['direct', 'express'] as const) {
    for (const firmwareTestVersion of [1, 255]) {
      const simulator = new VescSimulator(
        composeBoard({
          firmware: '7.00',
          firmwareTestVersion,
          refloat: '1.2.3',
          topology,
          canId: 42,
          requestPolicy: 'strict',
        })
      );
      const prefix = topology === 'express' ? [34, 42] : [];
      const replies = simulator.receive(encodeFrame(Uint8Array.of(...prefix, 36, 101, 20, 0, 0, 0, 2, 1)));
      assert.deepEqual([...decode(replies[0]!.frame)], [36, 101, 20, 1]);
      assert.equal(simulator.requests.at(-1)?.kind, 'refloat-control');
    }
  }
});

test('representative release families expose exact lighting state on direct and CAN routes', () => {
  for (const [refloat, topology, command] of [
    ['1.0.0', 'direct', 202],
    ['1.0.0', 'express', 202],
    ['1.1.0', 'direct', 202],
    ['1.1.0', 'express', 202],
    ['1.2.3', 'direct', 20],
    ['1.2.3', 'express', 20],
  ] as const) {
    const prefix = topology === 'express' ? [34, 42] : [];
    const request =
      command === 202
        ? Uint8Array.of(...prefix, 36, 101, command, 3, 2)
        : Uint8Array.of(...prefix, 36, 101, command, 0, 0, 0, 3, 2);
    const simulator = new VescSimulator(
      composeBoard({ firmware: refloat === '1.0.0' ? '6.02' : '6.06', refloat, topology, canId: 42 })
    );
    const response = simulator.receive(encodeFrame(request));
    assert.deepEqual([...decode(response[0]!.frame)], [36, 101, command, 2], `${refloat}-${topology}`);
  }
});

test('canonical safety matrix covers motion, fault, idempotence, flywheel exclusion, and reset isolation', () => {
  const normal = new RefloatControlModel('1.2.3', { fatalErrorLatched: true });
  assert.equal(normal.apply(Uint8Array.of(36, 101, 12, 0), { rideState: 'idle' }).accepted, true);
  assert.equal(normal.state.packageEnabled, true, 'idempotent package enable retains confirmed state');
  assert.deepEqual(normal.apply(Uint8Array.of(36, 101, 12, 1), { rideState: 'riding' }), {
    accepted: false,
    reason: 'invalid_state',
  });
  normal.state.fatalErrorActive = true;
  assert.deepEqual(normal.apply(Uint8Array.of(36, 101, 36, 1), { rideState: 'idle' }), {
    accepted: false,
    reason: 'invalid_state',
  });

  const flywheel = new RefloatControlModel('1.2.3', { flywheelEnabled: true });
  for (const payload of [
    Uint8Array.of(36, 101, 12, 1),
    Uint8Array.of(36, 101, 20, 0, 0, 0, 1, 0),
    Uint8Array.of(36, 101, 36, 1),
  ]) {
    assert.deepEqual(flywheel.apply(payload, { rideState: 'idle' }), {
      accepted: false,
      reason: 'invalid_state',
    });
  }
  assert.equal(flywheel.apply(Uint8Array.of(36, 101, 22, 0x80, 0, 0, 0, 0, 1), { rideState: 'idle' }).accepted, true);
  assert.equal(flywheel.state.flywheelEnabled, false);
  assert.equal(new RefloatControlModel('1.2.3').state.flywheelEnabled, false, 'reconnect creates fresh state');
});

test('selectable telemetry confirms package and fatal-clear transitions over direct and CAN', () => {
  for (const topology of ['direct', 'express'] as const) {
    const simulator = new VescSimulator(composeBoard({ firmware: '6.06', refloat: '1.3.0', topology, canId: 42 }));
    const prefix = topology === 'express' ? [34, 42] : [];
    const send = (bytes: number[]) => simulator.receive(encodeFrame(Uint8Array.from([...prefix, ...bytes])));
    const flags = () => {
      const payload = decode(send([36, 101, 33, 1, 0, 0, 0, 2, 0, 0, 0, 0])[0]!.frame);
      return new DataView(payload.buffer, payload.byteOffset, payload.byteLength).getUint32(16, false);
    };
    assert.ok(flags() & (1 << 20));
    send([36, 101, 36, 1]);
    assert.equal(flags() & (1 << 20), 0);
    send([36, 101, 12, 1]);
    assert.equal((flags() >>> 24) & 3, 0);
    send([36, 101, 12, 0]);
    assert.equal((flags() >>> 24) & 3, 2);
  }
});
