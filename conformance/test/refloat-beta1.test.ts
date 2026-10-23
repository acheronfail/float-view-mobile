import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { refloatCodecs } from '../fixture/refloat.ts';
import { RefloatControlModel } from '../fixture/refloat-controls.ts';
import { composeBoard } from '../fixture/boards.ts';
import { VescSimulator } from '../fixture/simulator.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { idleTelemetry } from '../fixture/telemetry.ts';

const source = JSON.parse(
  readFileSync(new URL('../../backends/fixtures/refloat/protocol-1.3.1-beta1.json', import.meta.url), 'utf8')
);

test('beta INFO and mixed alert history match the pinned source vectors', () => {
  const info = refloatCodecs['1.3.1-beta1'].infoPayload;
  assert.deepEqual([...info.slice(25, 28)], [1, 3, 1]);
  assert.equal(new TextDecoder().decode(info.slice(28, 33)), 'beta1');
  const expectedInfo = Buffer.from(source.infoV2Hex, 'hex');
  assert.deepEqual([...info.slice(0, 48)], [...expectedInfo.slice(0, 48)]);
  let ticks = 100;
  const control = new RefloatControlModel('1.3.1-beta1', { fatalErrorLatched: false }, () => ticks);
  control.recordFirmwareFault(4);
  control.recordImuTimeout(true);
  ticks++;
  control.recordImuTimeout(false);
  // Active mask is firmware only after timeout clears; history preserves both transitions.
  const response = control.alertsResponse(Uint8Array.of(36, 101, 35, 0, 0, 0, 0))!;
  const expected = Buffer.from(source.mixedAlertsHex, 'hex');
  expected[6] = 1;
  assert.deepEqual([...response], [...expected]);
  assert.equal(control.state.fatalErrorLatched, true);
  assert.throws(() => new RefloatControlModel('1.3.0').recordImuTimeout(true));
});

test('beta direct and CAN fixture routes retain selectable telemetry, schema and verified readback', () => {
  for (const topology of ['direct', 'express'] as const) {
    const board = composeBoard({
      firmware: '6.06',
      refloat: '1.3.1-beta1',
      topology,
      canId: 42,
      requestPolicy: 'strict',
    });
    const simulator = new VescSimulator(board);
    const request = (payload: Uint8Array) => {
      const routed = topology === 'express' ? Uint8Array.from([34, 42, ...payload]) : payload;
      const responses = simulator.receive(encodeFrame(routed));
      assert.equal(responses.length, 1);
      return new FrameAssembler().append(responses[0]!.frame)[0]!;
    };
    assert.equal(new TextDecoder().decode(request(Uint8Array.of(36, 101, 0, 2, 0)).slice(28, 33)), 'beta1');
    const fast = request(Uint8Array.of(36, 101, 33, 1, 0x7f, 0xff, 0xbf, 0xc3, 0, 0, 0, 0));
    assert.equal(fast[2], 33);
    const root = new URL('../../backends/fixtures/refloat-configuration/', import.meta.url);
    const write = readFileSync(new URL('values/1.3.1-beta1-write.bin', root));
    assert.deepEqual([...request(Uint8Array.from([95, 0, ...write]))], [95]);
    const expected = readFileSync(new URL('values/1.3.1-beta1-readback.bin', root));
    assert.deepEqual([...request(Uint8Array.of(93, 0))], [93, 0, ...expected]);
    assert.ok(
      refloatCodecs['1.3.1-beta1'].respond(Uint8Array.of(36, 101, 33, 1, 0x7f, 0xff, 0xbf, 0xc3, 0, 0, 0, 0), {
        ...idleTelemetry,
        stopCondition: 'imuTimeout',
      })
    );
  }
});
