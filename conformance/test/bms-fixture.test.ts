import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  bmsFailureModes,
  encodeBmsFirmwarePayload,
  encodeBmsValuesPayload,
  encodeVbms32FirmwarePayload,
  InteractiveBmsSource,
  typicalBmsValues,
  type BmsConfiguration,
} from '../fixture/bms.ts';
import { directThor, expressThor } from '../fixture/boards.ts';
import { encodeFrame, FrameAssembler } from '../fixture/frame.ts';
import { VescSimulator } from '../fixture/simulator.ts';
import { canonicalCanBms, softwareBmsScenarios } from '../fixture/scenarios.ts';

const source = JSON.parse(
  readFileSync(fileURLToPath(new URL('../../backends/fixtures/vesc-bms/source-derived.json', import.meta.url)), 'utf8')
) as { cases: Array<{ name: string; payloadHex: string }> };
const fixture = (name: string) => source.cases.find((item) => item.name === name)?.payloadHex;
const hex = (value: Uint8Array) => Buffer.from(value).toString('hex');

const canBms: BmsConfiguration = {
  canId: 42,
  firmware: {
    major: 6,
    minor: 0,
    hardwareName: 'VBMS',
    hardwareType: 1,
    payload: encodeBmsFirmwarePayload(
      'VBMS',
      Uint8Array.from({ length: 12 }, (_, index) => index * 0x11)
    ),
  },
  values: typicalBmsValues,
  tail: { kind: 'extended', dataVersion: 1, status: 'OK' },
};

const localBms: BmsConfiguration = {
  firmware: canBms.firmware,
  values: typicalBmsValues,
  tail: { kind: 'legacy' },
};

test('source-faithful VBMS encoders reproduce immutable firmware and values fixtures', () => {
  assert.equal(hex(canBms.firmware.payload), fixture('vbms-firmware-6.00'));
  assert.equal(
    hex(encodeVbms32FirmwarePayload(Uint8Array.of(0, 0x11, 0x22, 0x33, 0x44, 0x55))),
    fixture('vbms32-firmware-6.06')
  );
  assert.equal(hex(canonicalCanBms.firmware.payload), fixture('vbms32-firmware-6.06'));
  assert.equal(hex(encodeBmsValuesPayload(typicalBmsValues)), fixture('vbms-values-legacy-typical'));
  assert.equal(
    hex(encodeBmsValuesPayload(typicalBmsValues, { kind: 'extended', dataVersion: 1, status: 'OK' })),
    fixture('vbms-values-extended-typical')
  );
});

test('simulator composes independent direct-controller and CAN-VBMS routes', () => {
  const simulator = new VescSimulator({ ...directThor, bmsNodes: [canBms] });
  assert.deepEqual([...payload(simulator, [62])], [62, 42]);
  assert.equal(hex(payload(simulator, [34, 42, 0])), fixture('vbms-firmware-6.00'));
  assert.equal(hex(payload(simulator, [34, 42, 96])), fixture('vbms-values-extended-typical'));
  assert.throws(() => simulator.receive(encodeFrame(Uint8Array.of(34, 42, 97))), /unsupported BMS command/);
});

test('simulator composes a direct VBMS gateway with a CAN controller', () => {
  const simulator = new VescSimulator({
    ...expressThor,
    transport: localBms.firmware,
    localBms,
    bmsNodes: [],
  });
  assert.equal(hex(payload(simulator, [96])), fixture('vbms-values-legacy-typical'));
  assert.deepEqual([...payload(simulator, [62])], [62, 1]);
  assert.equal(payload(simulator, [34, 1, 0])[0], 0);
});

test('canonical BMS software scenario catalog covers topology, selection, bounds, and lifecycle', () => {
  assert.deepEqual(Object.keys(softwareBmsScenarios), [
    'direct-controller-plus-can-vbms',
    'express-plus-controller-and-vbms',
    'direct-vbms-plus-can-controller',
    'no-bms',
    'unique-automatic-selection',
    'configured-selection-without-ping',
    'wrong-preference',
    'multiple-ambiguity',
    'id-overflow',
    'candidate-overflow',
    'loss',
    'recovery',
    'reconnect',
    'disconnect',
  ]);

  for (const scenario of Object.values(softwareBmsScenarios)) {
    assert.equal(scenario.name.length > 0, true);
    assert.equal(scenario.board.requestPolicy, 'interactive');
  }

  const directController = new VescSimulator(softwareBmsScenarios['direct-controller-plus-can-vbms'].board);
  assert.deepEqual([...payload(directController, [62])], [62, 42]);
  assert.equal(payload(directController, [34, 42, 96])[0], 96);

  const express = new VescSimulator(softwareBmsScenarios['express-plus-controller-and-vbms'].board);
  assert.deepEqual([...payload(express, [62])], [62, 1, 42]);
  assert.equal(payload(express, [34, 1, 0])[0], 0);
  assert.equal(payload(express, [34, 42, 96])[0], 96);

  const gateway = new VescSimulator(softwareBmsScenarios['direct-vbms-plus-can-controller'].board);
  assert.equal(payload(gateway, [96])[0], 96);
  assert.equal(payload(gateway, [34, 1, 0])[0], 0);

  const omitted = new VescSimulator(softwareBmsScenarios['configured-selection-without-ping'].board);
  assert.deepEqual([...payload(omitted, [62])], [62, 1]);
  assert.equal(payload(omitted, [34, 42, 96])[0], 96);

  const noBms = new VescSimulator(softwareBmsScenarios['no-bms'].board);
  assert.equal(noBms.receive(encodeFrame(Uint8Array.of(34, 42, 96))).length, 0);

  const loss = new VescSimulator(softwareBmsScenarios.loss.board);
  assert.equal(loss.receive(encodeFrame(Uint8Array.of(34, 42, 96))).length, 0);
});

test('deterministic interactive BMS values are coherent, bounded, and request-driven', () => {
  const settings = {
    seed: 17,
    cellCount: 18,
    temperatureCount: 4,
    stateOfCharge: 0.8,
    inputCurrentA: -4,
    reportedCanId: 42,
  };
  const first = new InteractiveBmsSource(settings);
  const second = new InteractiveBmsSource(settings);
  const a = first.next();
  const b = first.next();
  assert.deepEqual(a, second.next());
  assert.notDeepEqual(a, b);
  assert.equal(a.cellVoltagesV.length, 18);
  assert.equal(a.balancing.length, 18);
  assert.equal(a.temperaturesC.length, 4);
  assert.ok(Math.abs(a.totalVoltageV - a.cellVoltagesV.reduce((sum, value) => sum + value, 0)) < 0.01);
  assert.ok(b.lifetimeChargeAmpHoursAh > a.lifetimeChargeAmpHoursAh);
  assert.throws(() => new InteractiveBmsSource({ ...settings, cellCount: 51 }), /cell count/);
});

test('malformed BMS response table covers every bounded decoder rejection surface', () => {
  const baseline = payload(new VescSimulator({ ...directThor, bmsNodes: [canBms] }), [34, 42, 96]);
  const malformedModes = bmsFailureModes.filter((mode) => mode !== 'none' && mode !== 'timeout');
  assert.deepEqual(malformedModes, [
    'excessive-cells',
    'excessive-temperatures',
    'invalid-balancing',
    'truncate-fixed',
    'truncate-cells',
    'truncate-balancing',
    'truncate-temperatures',
    'truncate-tail',
    'non-finite-auto-float',
    'reported-id-mismatch',
    'unterminated-status',
    'oversized-status',
    'invalid-status-utf8',
    'unexpected-remainder',
  ]);
  for (const failureMode of malformedModes) {
    const malformed = payload(
      new VescSimulator({ ...directThor, bmsNodes: [{ ...canBms, failureMode }] }),
      [34, 42, 96]
    );
    assert.notDeepEqual(malformed, baseline, failureMode);
    assert.equal(malformed[0], 96, failureMode);
  }
  const timedOut = new VescSimulator({ ...directThor, bmsNodes: [{ ...canBms, failureMode: 'timeout' }] });
  assert.equal(timedOut.receive(encodeFrame(Uint8Array.of(34, 42, 96))).length, 0);
});

function payload(simulator: VescSimulator, request: number[]): Uint8Array {
  const response = simulator.receive(encodeFrame(Uint8Array.from(request)));
  assert.equal(response.length, 1);
  const frames = new FrameAssembler().append(response[0]!.frame);
  assert.equal(frames.length, 1);
  return frames[0]!;
}
