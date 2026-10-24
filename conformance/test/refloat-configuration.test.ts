import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { composeBoard } from '../fixture/boards.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { RefloatConfigurationModel, type StableRefloatConfigurationVersion } from '../fixture/refloat-configuration.ts';
import { VescSimulator } from '../fixture/simulator.ts';
import { fixtureScenarios } from '../fixture/scenarios.ts';

const versions: readonly StableRefloatConfigurationVersion[] = [
  '1.0.0',
  '1.0.1',
  '1.1.0',
  '1.1.1',
  '1.1.2',
  '1.2.0',
  '1.2.1',
  '1.2.2',
  '1.2.3',
  '1.3.0',
];
const fixtureRoot = new URL('../../backends/fixtures/refloat-configuration/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('manifest.json', fixtureRoot), 'utf8')) as {
  releases: Array<{
    version: StableRefloatConfigurationVersion;
    schemaFingerprintSha256: string;
    wireSignature: number;
    files: { schema: string; default: string; current: string; acceptedWrite: string; expectedReadback: string };
  }>;
};

test('semantic model serves every exact stable release and applies disposable writes atomically', () => {
  for (const version of versions) {
    const release = manifest.releases.find((item) => item.version === version)!;
    const model = new RefloatConfigurationModel(version);
    assert.deepEqual(model.respond(Uint8Array.of(94, 0), true), response(94, release.files.default));
    assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), response(93, release.files.current));
    const write = bytes(release.files.acceptedWrite);
    assert.deepEqual(model.respond(Uint8Array.from([95, 0, ...write]), true), Uint8Array.of(95));
    assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), response(93, release.files.expectedReadback));
    assert.deepEqual(model.respond(Uint8Array.from([95, 0, ...bytes(release.files.current)]), true), Uint8Array.of(95));
    const restored = response(93, release.files.current);
    const readback = bytes(release.files.expectedReadback);
    for (let index = 0; index < write.length; index += 1) {
      if (write[index] !== readback[index]) restored[index + 2] = readback[index]!;
    }
    assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), restored);
    assert.equal(model.snapshot().fingerprint, release.schemaFingerprintSha256);
    assert.equal(model.snapshot().wireSignature, release.wireSignature);
    model.reset();
    assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), response(93, release.files.current));
  }
});

test('arbitrary complete writes persist across repeated edits on every release and route', () => {
  for (const version of versions) {
    for (const topology of ['direct', 'express'] as const) {
      const simulator = new VescSimulator(
        composeBoard({ firmware: '6.06', refloat: version, topology, canId: 1, requestPolicy: 'strict' })
      );
      const route = (payload: Uint8Array) => (topology === 'direct' ? payload : Uint8Array.from([34, 1, ...payload]));
      const current = bytes(`values/${version}-current.bin`);
      const edited = current.slice();
      const view = new DataView(edited.buffer);
      // Audited wire order starts with kp (scale 10), kp2 (scale 100).
      view.setInt16(4, 220);
      view.setInt16(6, 75);
      for (const kp of [220, 235, 200]) {
        view.setInt16(4, kp);
        assert.deepEqual(decode(simulator, route(Uint8Array.from([95, 0, ...edited]))), Uint8Array.of(95));
        const expected = edited.slice();
        if (!version.startsWith('1.0.')) expected[expected.length - 1] = 0; // meta.is_default
        assert.deepEqual(decode(simulator, route(Uint8Array.of(93, 0))), Uint8Array.from([93, 0, ...expected]));
        assert.deepEqual(decode(simulator, route(Uint8Array.of(94, 0))), response(94, `values/${version}-default.bin`));
      }
      simulator.reset();
      assert.deepEqual(decode(simulator, route(Uint8Array.of(93, 0))), Uint8Array.from([93, 0, ...current]));
    }
  }
});

test('arbitrary writes validate signature, complete layout, numeric ranges, booleans and enums atomically', () => {
  const model = new RefloatConfigurationModel('1.3.0');
  const edited = bytes('values/1.3.0-current.bin');
  // Source-derived 1.3.0 offsets: fault_pitch:u8, footbeep:bool, front mode:enum.
  edited[42] = 65;
  edited[48] = 0;
  edited[186] = 2;
  assert.deepEqual(model.respond(Uint8Array.from([95, 0, ...edited]), true), Uint8Array.of(95));
  const baseline = model.respond(Uint8Array.of(93, 0), true);
  const changed = (offset: number, value: number) => {
    const copy = edited.slice();
    copy[offset] = value;
    return copy;
  };
  for (const [values, reason] of [
    [edited.subarray(0, edited.length - 1), /length mismatch/],
    [Uint8Array.from([...edited, 0]), /length mismatch/],
    [changed(0, edited[0]! ^ 1), /signature mismatch/],
    [changed(4, 0x7f), /invalid value for kp$/],
    [changed(42, 0), /invalid value for fault_pitch$/],
    [changed(48, 2), /invalid value for is_footbeep_enabled$/],
    [changed(186, 127), /invalid value for leds.front.mode$/],
  ] as const) {
    assert.equal(model.respond(Uint8Array.from([95, 0, ...values]), true), undefined);
    assert.match(model.rejectionReason!, reason);
    assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), baseline);
  }
  assert.equal(model.counters.write, 1);
  const fork = model.fork();
  fork.reset();
  assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), baseline, 'resetting a fork mutated its owner');
});

test('rejection reasons distinguish unsafe state, indexes and malformed schema chunks', () => {
  const model = new RefloatConfigurationModel('1.3.0');
  for (const [payload, safety, reason] of [
    [Uint8Array.of(95, 0), 'riding', /controller is not idle/],
    [Uint8Array.of(93, 0), 'flywheel', /flywheel mode is active/],
    [Uint8Array.of(95, 1), true, /index must be 0/],
    [Uint8Array.of(92, 0), true, /exactly 10 bytes/],
    [Uint8Array.from([92, 0, ...i32(401), ...i32(0)]), true, /chunk length/],
    [Uint8Array.from([92, 0, ...i32(400), ...i32(99_999)]), true, /offset is out of bounds/],
  ] as const) {
    assert.equal(model.respond(payload, safety), undefined);
    assert.match(model.rejectionReason!, reason);
  }
  assert.equal(model.counters.write, 0);
  model.reset();
  assert.equal(model.rejectionReason, undefined);
});

test('commands 92–95 preserve exact bytes over direct and once-forwarded CAN routes', () => {
  for (const topology of ['direct', 'express'] as const) {
    const board = composeBoard({ firmware: '6.06', refloat: '1.3.0', topology, canId: 42, requestPolicy: 'strict' });
    const simulator = new VescSimulator(board);
    const route = (payload: Uint8Array) => (topology === 'direct' ? payload : Uint8Array.from([34, 42, ...payload]));
    const schema = bytes('schemas/1.3.0.qcompress');
    const first = decode(simulator, route(Uint8Array.from([92, 0, ...i32(400), ...i32(0)])));
    assert.deepEqual(first, Uint8Array.from([92, 0, ...i32(schema.length), ...i32(0), ...schema.slice(0, 400)]));
    const tailOffset = schema.length - 17;
    const tail = decode(simulator, route(Uint8Array.from([92, 0, ...i32(400), ...i32(tailOffset)])));
    assert.deepEqual(
      tail,
      Uint8Array.from([92, 0, ...i32(schema.length), ...i32(tailOffset), ...schema.slice(tailOffset)])
    );
    assert.deepEqual(decode(simulator, route(Uint8Array.of(93, 0))), response(93, 'values/1.3.0-current.bin'));
    assert.deepEqual(decode(simulator, route(Uint8Array.of(94, 0))), response(94, 'values/1.3.0-default.bin'));
    assert.deepEqual(
      decode(simulator, route(Uint8Array.from([95, 0, ...bytes('values/1.3.0-write.bin')]))),
      Uint8Array.of(95)
    );
    assert.deepEqual(decode(simulator, route(Uint8Array.of(93, 0))), response(93, 'values/1.3.0-readback.bin'));
    assert.ok(simulator.requests.every((request) => request.canId === (topology === 'direct' ? undefined : 42)));
    assert.deepEqual(
      simulator.requests.map((request) => request.kind),
      [
        'refloat-configuration-schema',
        'refloat-configuration-schema',
        'refloat-configuration-current',
        'refloat-configuration-default',
        'refloat-configuration-write',
        'refloat-configuration-current',
      ]
    );
  }
});

test('physical control boards advertise and serve the required configuration owner', () => {
  for (const name of ['control-direct', 'control-can'] as const) {
    const controller = fixtureScenarios[name].board.controllers[0]!;
    assert.equal(controller.firmware.payload[26], 1, `${name} firmware configuration count`);
    assert.ok(controller.refloatConfiguration, `${name} configuration model`);
    assert.deepEqual(
      controller.refloatConfiguration.respond(Uint8Array.of(93, 0), true),
      response(93, 'values/1.3.0-current.bin')
    );
  }
});

test('configuration model rejects unknown indexes, invalid writes, unsafe current reads, and malformed chunks without state change', () => {
  const model = new RefloatConfigurationModel('1.0.0');
  const baseline = model.respond(Uint8Array.of(93, 0), true);
  assert.equal(model.respond(Uint8Array.of(93, 1), true), undefined);
  assert.equal(model.respond(Uint8Array.of(93, 0), false), undefined);
  assert.equal(model.respond(Uint8Array.of(94, 1), true), undefined);
  assert.equal(model.respond(Uint8Array.of(95, 0, 1, 2, 3), true), undefined);
  assert.equal(model.respond(Uint8Array.from([92, 0, ...i32(401), ...i32(0)]), true), undefined);
  assert.equal(model.respond(Uint8Array.from([92, 0, ...i32(100), ...i32(99_999)]), true), undefined);
  assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), baseline);
  assert.equal(model.counters.write, 0);
  assert.equal(model.counters.rejected, 6);
});

test('Refloat 1.0 collision retains two exact fingerprints with the same INFO and wire signature', () => {
  const first = new RefloatConfigurationModel('1.0.0').snapshot();
  const second = new RefloatConfigurationModel('1.0.1').snapshot();
  assert.equal(first.wireSignature, second.wireSignature);
  assert.notEqual(first.fingerprint, second.fingerprint);
});

test('test firmware permits configuration reads and writes on direct and CAN routes', () => {
  for (const topology of ['direct', 'express'] as const) {
    for (const firmwareTestVersion of [1, 255]) {
      const simulator = new VescSimulator(
        composeBoard({
          firmware: '6.06',
          firmwareTestVersion,
          refloat: '1.2.3',
          topology,
          canId: 42,
          requestPolicy: 'strict',
        })
      );
      const prefix = topology === 'express' ? [34, 42] : [];
      assert.deepEqual(decode(simulator, Uint8Array.of(...prefix, 93, 0)), response(93, 'values/1.2.3-current.bin'));
      assert.deepEqual(
        decode(simulator, Uint8Array.from([...prefix, 95, 0, ...bytes('values/1.2.3-write.bin')])),
        Uint8Array.of(95)
      );
      assert.deepEqual(decode(simulator, Uint8Array.of(...prefix, 93, 0)), response(93, 'values/1.2.3-readback.bin'));
    }
  }
});

test('strict fixture rejects riding writes/current reads, unsafe app-data, and malformed commands', () => {
  const riding = new VescSimulator(
    composeBoard({
      firmware: '6.06',
      refloat: '1.2.3',
      topology: 'direct',
      requestPolicy: 'strict',
      telemetry: {
        phase: () => 'riding',
        next: () => {
          throw new Error('not needed');
        },
      },
    })
  );
  assert.throws(() => decode(riding, Uint8Array.of(93, 0)), /Rejected Refloat configuration/);
  assert.throws(
    () => decode(riding, Uint8Array.from([95, 0, ...bytes('values/1.2.3-write.bin')])),
    /Rejected Refloat configuration/
  );
  assert.doesNotThrow(() => decode(riding, Uint8Array.of(94, 0)));

  const safe = new VescSimulator(
    composeBoard({ firmware: '6.06', refloat: '1.2.3', topology: 'direct', requestPolicy: 'strict' })
  );
  for (const payload of [
    Uint8Array.of(93, 1),
    Uint8Array.of(95, 0, 1),
    Uint8Array.of(36, 101, 99),
    Uint8Array.of(13),
    Uint8Array.of(16),
    Uint8Array.of(17),
    Uint8Array.of(18),
    Uint8Array.of(255),
  ]) {
    assert.throws(() => decode(safe, payload));
  }
});

function decode(simulator: VescSimulator, payload: Uint8Array) {
  const responses = simulator.receive(encodeFrame(payload));
  assert.equal(responses.length, 1);
  const frames = new FrameAssembler().append(responses[0]!.frame);
  assert.equal(frames.length, 1);
  return frames[0]!;
}

function response(command: 93 | 94, path: string) {
  return Uint8Array.from([command, 0, ...bytes(path)]);
}

function bytes(path: string) {
  return new Uint8Array(readFileSync(new URL(path, fixtureRoot)));
}

function i32(value: number) {
  return [value >>> 24, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff];
}
