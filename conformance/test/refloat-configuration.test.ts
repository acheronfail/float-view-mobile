import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { composeBoard } from '../fixture/boards.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { RefloatConfigurationModel, type StableRefloatConfigurationVersion } from '../fixture/refloat-configuration.ts';
import { VescSimulator } from '../fixture/simulator.ts';

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
    assert.equal(model.snapshot().fingerprint, release.schemaFingerprintSha256);
    assert.equal(model.snapshot().wireSignature, release.wireSignature);
    model.reset();
    assert.deepEqual(model.respond(Uint8Array.of(93, 0), true), response(93, release.files.current));
  }
});

test('commands 92–95 preserve exact bytes over direct and once-forwarded CAN routes', () => {
  for (const topology of ['direct', 'express'] as const) {
    const board = composeBoard({ firmware: '6.06', refloat: '1.2.3', topology, canId: 42, requestPolicy: 'strict' });
    const simulator = new VescSimulator(board);
    const route = (payload: Uint8Array) => (topology === 'direct' ? payload : Uint8Array.from([34, 42, ...payload]));
    const schema = bytes('schemas/1.2.3.qcompress');
    const first = decode(simulator, route(Uint8Array.from([92, 0, ...i32(400), ...i32(0)])));
    assert.deepEqual(first, Uint8Array.from([92, 0, ...i32(schema.length), ...i32(0), ...schema.slice(0, 400)]));
    const tailOffset = schema.length - 17;
    const tail = decode(simulator, route(Uint8Array.from([92, 0, ...i32(400), ...i32(tailOffset)])));
    assert.deepEqual(
      tail,
      Uint8Array.from([92, 0, ...i32(schema.length), ...i32(tailOffset), ...schema.slice(tailOffset)])
    );
    assert.deepEqual(decode(simulator, route(Uint8Array.of(93, 0))), response(93, 'values/1.2.3-current.bin'));
    assert.deepEqual(decode(simulator, route(Uint8Array.of(94, 0))), response(94, 'values/1.2.3-default.bin'));
    assert.deepEqual(
      decode(simulator, route(Uint8Array.from([95, 0, ...bytes('values/1.2.3-write.bin')]))),
      Uint8Array.of(95)
    );
    assert.deepEqual(decode(simulator, route(Uint8Array.of(93, 0))), response(93, 'values/1.2.3-readback.bin'));
    assert.ok(simulator.requests.every((request) => request.canId === (topology === 'direct' ? undefined : 42)));
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

test('strict fixture rejects test firmware, riding writes/current reads, unsafe app-data, and malformed commands', () => {
  const testFirmware = new VescSimulator(
    composeBoard({
      firmware: '6.06',
      firmwareTestVersion: 1,
      refloat: '1.2.3',
      topology: 'direct',
      requestPolicy: 'strict',
    })
  );
  assert.throws(() => decode(testFirmware, Uint8Array.of(93, 0)), /test firmware/);
  assert.throws(
    () => decode(testFirmware, Uint8Array.from([95, 0, ...bytes('values/1.2.3-write.bin')])),
    /test firmware/
  );

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
