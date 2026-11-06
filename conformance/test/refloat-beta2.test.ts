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
  readFileSync(new URL('../../backends/fixtures/refloat/protocol-1.3.1-beta2.json', import.meta.url), 'utf8')
);

test('beta INFO and mixed alert history match the pinned source vectors', () => {
  const info = refloatCodecs['1.3.1-beta2'].infoPayload;
  assert.deepEqual([...info.slice(25, 28)], [1, 3, 1]);
  assert.equal(new TextDecoder().decode(info.slice(28, 33)), 'beta2');
  const expectedInfo = Buffer.from(source.infoV2Hex, 'hex');
  assert.deepEqual([...info.slice(0, 48)], [...expectedInfo.slice(0, 48)]);
  let ticks = 100;
  const control = new RefloatControlModel('1.3.1-beta2', { fatalErrorLatched: false }, () => ticks);
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

test('audited release direct and CAN routes retain telemetry, schema and verified readback', () => {
  for (const version of ['1.3.1-beta2', '1.3.1-beta3', '1.3.1'] as const) {
    for (const topology of ['direct', 'express'] as const) {
      const board = composeBoard({
        firmware: '6.06',
        refloat: version,
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
      const info = request(Uint8Array.of(36, 101, 0, 2, 0));
      assert.deepEqual([...info.slice(25, 28)], [1, 3, 1]);
      assert.equal(Buffer.from(info.slice(28, 48)).toString().replaceAll('\0', ''), version.split('-')[1] ?? '1');
      const fast = request(Uint8Array.of(36, 101, 33, 1, 0x7f, 0xff, 0xbf, 0xc3, 0, 0, 0, 0));
      assert.equal(fast[2], 33);
      const root = new URL('../../backends/fixtures/refloat-configuration/', import.meta.url);
      const write = readFileSync(new URL('values/1.3.1-beta2-write.bin', root));
      assert.deepEqual([...request(Uint8Array.from([95, 0, ...write]))], [95]);
      const expected = readFileSync(new URL('values/1.3.1-beta2-readback.bin', root));
      assert.deepEqual([...request(Uint8Array.of(93, 0))], [93, 0, ...expected]);
      assert.ok(
        refloatCodecs[version].respond(Uint8Array.of(36, 101, 33, 1, 0x7f, 0xff, 0xbf, 0xc3, 0, 0, 0, 0), {
          ...idleTelemetry,
          stopCondition: 'imuTimeout',
        })
      );
    }
  }
});

test('audited beta3 changes identity and shares beta2 protocol/configuration fixtures', () => {
  const beta2 = refloatCodecs['1.3.1-beta2'];
  const beta3 = refloatCodecs['1.3.1-beta3'];
  const expected = beta2.infoPayload.slice();
  expected[32] = '3'.charCodeAt(0);
  assert.deepEqual(beta3.infoPayload, expected);
  assert.equal(beta3.telemetryCommand, beta2.telemetryCommand);
  assert.equal(beta3.controlFamily, beta2.controlFamily);
  assert.equal(beta3.alertProfile, beta2.alertProfile);
  const manifest = JSON.parse(
    readFileSync(new URL('../../backends/fixtures/refloat-configuration/manifest.json', import.meta.url), 'utf8')
  );
  const release = (version: string) => manifest.releases.find((item: { version: string }) => item.version === version);
  assert.equal(release('1.3.1-beta3').commit, 'e6388f75e63919bc42a3e10cecb68824ac413ae4');
  assert.deepEqual(release('1.3.1-beta3').files, release('1.3.1-beta2').files);
});

test('stable release keeps exact identity and source provenance while sharing audited evidence', () => {
  const stable = refloatCodecs['1.3.1'];
  const beta3 = refloatCodecs['1.3.1-beta3'];
  const expected = beta3.infoPayload.slice();
  expected.fill(0, 28, 48);
  expected[28] = '1'.charCodeAt(0);
  assert.deepEqual(stable.infoPayload, expected);
  assert.equal(stable.telemetryCommand, beta3.telemetryCommand);
  assert.equal(stable.controlFamily, beta3.controlFamily);
  assert.equal(stable.alertProfile, beta3.alertProfile);
  const manifest = JSON.parse(
    readFileSync(new URL('../../backends/fixtures/refloat-configuration/manifest.json', import.meta.url), 'utf8')
  );
  const release = (version: string) => manifest.releases.find((item: { version: string }) => item.version === version);
  assert.equal(release('1.3.1').commit, 'a2b55d96f508653b0436b21410100aaf91f14b04');
  assert.equal(release('1.3.1').tag, 'v1.3.1');
  assert.equal(release('1.3.1').sourceSha256, release('1.3.1-beta3').sourceSha256);
  assert.deepEqual(release('1.3.1').files, release('1.3.1-beta3').files);
});
