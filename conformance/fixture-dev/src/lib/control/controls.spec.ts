import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import type { FixtureSnapshot } from '../contract.ts';
import {
  activateFromKeyboardOrAssistiveClick,
  beginPointerActivation,
  completePointerActivation,
} from './activation.ts';
import { canonicalCanId, controlAvailability, shouldReplaceSnapshot, startBoundedPolling } from './availability.ts';

test('CAN input validation accepts only canonical decimal controller identities', () => {
  assert.equal(canonicalCanId('0'), 0);
  assert.equal(canonicalCanId('42'), 42);
  assert.equal(canonicalCanId('253'), 253);
  for (const value of ['01', ' 1', '1 ', '-1', '1.5', '0x2a', '254', '255', '']) {
    assert.equal(canonicalCanId(value), undefined, value);
  }
});

test('control availability enforces powered-on ride transitions for every topology', () => {
  const snapshot = baseSnapshot();
  assert.equal(controlAvailability(snapshot).startRide, false);
  snapshot.serial = { status: 'attached', path: '/dev/fixture' };
  snapshot.power = 'on';
  snapshot.ble = 'advertising';
  const powered = controlAvailability(snapshot);
  assert.equal(powered.configure, false);
  assert.equal(powered.startRide, true);
  assert.equal(powered.stopRide, false);
  snapshot.ride = 'riding';
  assert.equal(controlAvailability(snapshot).startRide, false);
  assert.equal(controlAvailability(snapshot).stopRide, true);
  snapshot.configuration.topology = 'direct';
  assert.equal(controlAvailability(snapshot).stopRide, true);
});

test('primary pointer and keyboard activation each run exactly once', () => {
  let count = 0;
  const action = () => {
    count += 1;
  };
  const pending = beginPointerActivation({
    isPrimary: true,
    button: 0,
    pointerId: 7,
    preventDefault() {},
  } as PointerEvent);
  completePointerActivation(
    { isPrimary: true, button: 0, pointerId: 7, preventDefault() {} } as PointerEvent,
    pending,
    action
  );
  activateFromKeyboardOrAssistiveClick({ detail: 1 } as MouseEvent, action);
  assert.equal(count, 1);
  activateFromKeyboardOrAssistiveClick({ detail: 0 } as MouseEvent, action);
  assert.equal(count, 2);
  assert.equal(
    beginPointerActivation({ isPrimary: false, button: 0, pointerId: 8, preventDefault() {} } as PointerEvent),
    undefined
  );
  assert.equal(count, 2);
  const cancelled = beginPointerActivation({
    isPrimary: true,
    button: 0,
    pointerId: 9,
    preventDefault() {},
  } as PointerEvent);
  completePointerActivation(
    { isPrimary: true, button: 0, pointerId: 10, preventDefault() {} } as PointerEvent,
    cancelled,
    action
  );
  assert.equal(count, 2);
});

test('snapshot replacement is monotonic and polling cleanup cancels its timer', () => {
  const current = baseSnapshot();
  assert.equal(shouldReplaceSnapshot(current, { ...current, revision: 1 }), true);
  assert.equal(shouldReplaceSnapshot({ ...current, revision: 2 }, { ...current, revision: 1 }), false);
  let polled = 0;
  let cancelled: unknown;
  const stop = startBoundedPolling(
    () => {
      polled += 1;
    },
    500,
    ((callback: () => void, interval: number) => {
      assert.equal(interval, 500);
      callback();
      return 17;
    }) as typeof setInterval,
    ((timer: unknown) => {
      cancelled = timer;
    }) as typeof clearInterval
  );
  assert.equal(polled, 1);
  stop();
  assert.equal(cancelled, 17);
});

test('workbench renders persistent serial failures and request-attempt context', () => {
  const source = readFileSync(new URL('./ControlPanel.svelte', import.meta.url), 'utf8');
  assert.match(source, /snapshot\.serial\.error/);
  assert.match(source, /snapshot\.lastAttempt/);
  assert.match(source, /role="alert">Serial bridge:/);
  assert.match(source, /<fieldset disabled=\{!available\.configure \|\| control\.loading\}>/);
  assert.match(source, /<legend>VESC BMS telemetry<\/legend>/);
  assert.match(source, /BMS CAN \/ reported ID/);
  assert.match(source, /snapshot\.counters\.bms/);
});

function baseSnapshot(): FixtureSnapshot {
  return {
    revision: 0,
    serial: { status: 'detached' },
    configuration: {
      topology: 'express',
      canId: 1,
      firmware: '6.06',
      refloat: '1.2.3',
      bmsTopology: 'can',
      bmsCanId: 42,
      bmsVariant: 'extended',
      bmsFailureMode: 'none',
      bmsCellCount: 15,
      bmsTemperatureCount: 3,
      bmsStateOfCharge: 0.75,
      bmsInputCurrentA: 0,
    },
    scenario: {
      schemaVersion: 1,
      namespace: 'test',
      advertisedName: 'Fixture',
      topology: 'express',
      fragmentBytes: 244,
      controllers: [
        {
          alias: 'primary',
          uuid: '010101010101010101010101',
          canId: 1,
          firmware: '6.06',
          refloat: '1.2.3',
          pingVisible: true,
        },
      ],
      bmsNodes: [],
      discoveredCanIds: [],
      scene: 'idle',
      overrides: {},
    },
    frozen: false,
    pinnedFields: {},
    availableFields: [],
    power: 'off',
    ble: 'unavailable',
    ride: 'idle',
    counters: { writes: 0, responses: 0, telemetry: 0, bms: 0, dropped: 0 },
    refloatConfigurations: [],
    diagnostics: [],
  };
}
