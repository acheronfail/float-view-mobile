import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { composeBoard } from '../fixture/boards.ts';
import {
  ControllerShutdownModel,
  decodeControllerShutdown,
  type ControllerShutdownRejection,
} from '../fixture/controller-shutdown.ts';
import { encodeFrame } from '../fixture/frame.ts';
import { VescSimulator } from '../fixture/simulator.ts';

interface FixtureCase {
  name: string;
  version: string;
  request: number[];
  route?: 'direct' | 'can';
  canId?: number;
}

interface RejectionCase extends FixtureCase {
  selectedCanId?: number;
  firmwareTestVersion?: number;
  shutdownPending?: boolean;
  reason: ControllerShutdownRejection;
}

const path = fileURLToPath(new URL('../../backends/fixtures/vesc/controller-shutdown.json', import.meta.url));
const fixture = JSON.parse(readFileSync(path, 'utf8')) as {
  cases: FixtureCase[];
  rejections: RejectionCase[];
};

test('source-derived shutdown requests accept only exact direct and single-forward CAN packets', () => {
  for (const item of fixture.cases) {
    assert.deepEqual(
      decodeControllerShutdown(item.version, Uint8Array.from(item.request), {
        ...(item.route === 'can' ? { selectedCanId: item.canId } : {}),
      }),
      { accepted: true },
      item.name
    );
  }
  for (const item of fixture.rejections) {
    assert.deepEqual(
      decodeControllerShutdown(item.version, Uint8Array.from(item.request), {
        selectedCanId: item.selectedCanId,
        firmwareTestVersion: item.firmwareTestVersion,
        shutdownPending: item.shutdownPending,
      }),
      { accepted: false, reason: item.reason },
      item.name
    );
  }
});

test('shutdown requires idle non-flywheel state and shares the pending mutation gate', () => {
  for (const context of [
    { rideState: 'riding' as const, flywheel: false, mutationPending: false },
    { rideState: 'idle' as const, flywheel: true, mutationPending: false },
    { rideState: 'idle' as const, flywheel: false, mutationPending: true },
  ]) {
    const model = new ControllerShutdownModel();
    assert.deepEqual(model.apply({ version: '6.06', testVersion: 0 }, Uint8Array.of(156, 0, 0), context), {
      accepted: false,
      reason: 'invalid_state',
    });
  }
});

test('direct and CAN shutdown are one-way and selected-controller loss leaves Express available', () => {
  for (const [topology, request] of [
    ['direct', [156, 0, 0]],
    ['express', [34, 42, 156, 0, 0]],
  ] as const) {
    const simulator = new VescSimulator(
      composeBoard({ firmware: '6.06', refloat: '1.2.3', topology, canId: 42, requestPolicy: 'interactive' })
    );
    assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.from(request))), []);
    assert.equal(simulator.requests.at(-1)?.kind, 'controller-shutdown');
    assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.from(request))), []);
    if (topology === 'express') {
      assert.equal(simulator.receive(encodeFrame(Uint8Array.of(0))).length, 1, 'Express transport remains powered');
      assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(34, 42, 0))), []);
    } else {
      assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(0))), []);
    }
  }
});

test('unsupported controller hardware deterministically no-ops without claiming power-off', () => {
  const simulator = new VescSimulator(
    composeBoard({
      firmware: '7.00',
      refloat: '1.2.3',
      topology: 'direct',
      shutdownBehavior: 'no-op',
      requestPolicy: 'interactive',
    })
  );
  assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(156, 0, 0))), []);
  assert.equal(simulator.requests.at(-1)?.kind, 'controller-shutdown');
  assert.equal(simulator.receive(encodeFrame(Uint8Array.of(0))).length, 1);
  assert.deepEqual(simulator.receive(encodeFrame(Uint8Array.of(156, 0, 0))), []);
  assert.match(simulator.diagnostics.at(-1) ?? '', /invalid_state/);
});

test('recognised test firmware permits shutdown while unknown versions remain rejected', () => {
  for (const testVersion of [1, 255]) {
    for (const version of ['6.05', '6.06', '7.00']) {
      const model = new ControllerShutdownModel();
      assert.deepEqual(
        model.apply({ version, testVersion }, Uint8Array.of(156, 0, 0), {
          rideState: 'idle',
          flywheel: false,
          mutationPending: false,
        }),
        { accepted: true, disconnected: true }
      );
    }
    for (const version of ['6.02', '6.07', '7.01', '8.00']) {
      assert.deepEqual(
        decodeControllerShutdown(version, Uint8Array.of(156, 0, 0), {
          firmwareTestVersion: testVersion,
        }),
        { accepted: false, reason: 'unsupported_version' }
      );
    }
  }
});
