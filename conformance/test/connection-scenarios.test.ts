import assert from 'node:assert/strict';
import test from 'node:test';
import { MockBackend } from '../../frontend/backend-mock/api-v1.ts';
import type {
  AppState,
  ConnectionIssueCode,
  ControllerCandidate,
  Response,
} from '../../frontend/src/lib/backend/contract.ts';
import { directThor, expressExplicitThor } from '../fixture/boards.ts';
import { encodeFrame } from '../fixture/frame.ts';
import { softwareConnectionScenarios, type SoftwareConnectionScenarioName } from '../fixture/scenarios.ts';
import { VescSimulator } from '../fixture/simulator.ts';
import { assertState } from '../src/suite.ts';

const scenarioNames: SoftwareConnectionScenarioName[] = [
  'automatic-can-discovery',
  'private-hint-match',
  'private-hint-fallback',
  'configured-can-without-ping',
  'wrong-configured-can',
  'generic-gateway-forwarding',
  'mixed-can-nodes',
  'zero-candidates',
  'can-id-overflow',
  'candidate-overflow',
  'supported-unsupported-ambiguity',
  'selection-validation',
  'selection-atomic-persistence',
  'failed-retry',
  'reconnect-continuity',
  'frontend-recovery',
];

test('canonical connection scenario catalog is complete and internally named', () => {
  assert.deepEqual(Object.keys(softwareConnectionScenarios), scenarioNames);
  for (const name of scenarioNames) {
    assert.equal(softwareConnectionScenarios[name].name, name);
  }
});

test('canonical discovery scenarios exercise configured, hinted, mixed, and bounded CAN paths', () => {
  const automatic = simulator('automatic-can-discovery');
  request(automatic, [0]);
  assert.deepEqual(payload(request(automatic, [62])), [62, 1]);
  assert.equal(request(automatic, [34, 1, 0]).request.canId, 1);

  const hinted = simulator('private-hint-match');
  request(hinted, [0]);
  assert.equal(request(hinted, [34, 42, 0]).request.canId, 42);
  assert.equal(
    hinted.requests.some((item) => item.kind === 'can-ping'),
    false
  );

  const fallback = simulator('private-hint-fallback');
  request(fallback, [0]);
  assert.equal(payload(request(fallback, [34, 7, 0]))[2], 5);
  assert.deepEqual(payload(request(fallback, [62])), [62, 7, 42]);
  assert.equal(payload(request(fallback, [34, 42, 0]))[2], 6);

  const configured = simulator('configured-can-without-ping');
  request(configured, [0]);
  assert.equal(request(configured, [34, 42, 0]).request.canId, 42);
  assert.equal(
    configured.requests.some((item) => item.kind === 'can-ping'),
    false
  );

  const wrong = simulator('wrong-configured-can');
  request(wrong, [0]);
  assert.deepEqual(wrong.receive(encodeFrame(Uint8Array.of(34, 7, 0))), []);
  assert.match(wrong.diagnostics.at(-1) ?? '', /Unknown CAN target 7/);

  const generic = simulator('generic-gateway-forwarding');
  assert.equal(payload(request(generic, [0]))[2], 5);
  assert.equal(request(generic, [34, 1, 0]).request.kind, 'controller-firmware');

  const mixed = simulator('mixed-can-nodes');
  assert.deepEqual(payload(request(mixed, [62])), [62, 42, 7, 42, 254, 255]);
  assert.equal(payload(request(mixed, [34, 7, 0]))[2], 5);
  assert.equal(payload(request(mixed, [34, 42, 0]))[2], 6);

  const zero = simulator('zero-candidates');
  assert.deepEqual(payload(request(zero, [62])), [62, 7]);
  assert.deepEqual(payload(request(zero, [34, 7, 0])).slice(0, 3), [0, 6, 5]);

  const excessiveIDs = simulator('can-id-overflow');
  assert.equal(payload(request(excessiveIDs, [62])).length, 66);

  const excessiveControllers = simulator('candidate-overflow');
  assert.equal(payload(request(excessiveControllers, [62])).length, 18);
  for (let canId = 0; canId < 17; canId += 1) {
    assert.equal(request(excessiveControllers, [34, canId, 0]).request.kind, 'controller-firmware');
  }

  const ambiguous = simulator('supported-unsupported-ambiguity');
  assert.deepEqual(payload(request(ambiguous, [62])), [62, 7, 42]);
  assert.equal(payload(request(ambiguous, [34, 7, 0]))[2], 3);
  assert.equal(payload(request(ambiguous, [34, 42, 0]))[2], 6);
});

test('canonical route assertions distinguish direct and CAN polling and reject unsafe commands', () => {
  const direct = new VescSimulator(directThor);
  assert.equal(request(direct, [36, 101, 31]).request.canId, undefined);
  assert.throws(() => request(direct, [6]), /Unsafe or unsupported forwarded command/);

  const can = new VescSimulator(expressExplicitThor);
  assert.equal(request(can, [34, 42, 36, 101, 31]).request.canId, 42);
  assert.throws(() => request(can, [6]), /Unsafe or unsupported VESC command/);

  const bounded = simulator('wrong-configured-can');
  for (let canId = 0; canId < 110; canId += 1) {
    bounded.receive(encodeFrame(Uint8Array.of(34, canId % 42, 0)));
  }
  assert.equal(bounded.diagnostics.length, 50);
});

test('authoritative snapshots enforce pre-ready identity, route, ordering, bounds, and live telemetry shape', () => {
  const backend = connectedBackend();
  backend.simulateIdentifying();
  const identifying = state(backend);
  assertState(identifying);
  assert.equal(identifying.activeConnection?.state, 'identifying');
  assert.equal(identifying.activeConnection?.savedDeviceId, 'saved-board');
  assert.equal(identifying.activeConnection?.activeSession, null);
  assert.equal(identifying.activeConnection?.controllerConnection, null);

  const readyBackend = connectedBackend();
  const direct = state(readyBackend);
  assertState(direct);
  assert.deepEqual(direct.activeConnection?.controllerConnection, {
    kind: 'direct',
  });
  assert.ok(direct.activeConnection?.activeSession);

  readyBackend.simulateCanReady(42);
  const can = state(readyBackend);
  assertState(can);
  assert.deepEqual(can.activeConnection?.controllerConnection, {
    kind: 'can',
    canId: 42,
  });

  readyBackend.simulateControllerSelection(controllerCandidates);
  const selecting = state(readyBackend);
  assertState(selecting);
  assert.deepEqual(
    selecting.activeConnection?.controllerCandidates.map((item) => item.canId),
    [7, 42]
  );
  assert.deepEqual(Object.keys(selecting.activeConnection!.controllerCandidates[0]!).sort(), [
    'canId',
    'firmwareName',
    'hardwareName',
    'vescSupported',
    'vescVersion',
  ]);
  assert.ok(!JSON.stringify(selecting.activeConnection?.controllerCandidates).toLowerCase().includes('uuid'));
});

test('selection errors are exact and accepted selection atomically merges the current profile', () => {
  const backend = connectedBackend();
  backend.simulateControllerSelection(controllerCandidates);
  let id = 10;
  for (const params of [
    {},
    { deviceId: 'mock-board' },
    { deviceId: 'mock-board', canId: -1 },
    { deviceId: 'mock-board', canId: 1.5 },
    { deviceId: 'mock-board', canId: 254 },
    { deviceId: 'mock-board', canId: '42' },
  ]) {
    assertError(handle(backend, id++, 'device.selectController', params), 'invalid_request');
  }
  for (const params of [
    { deviceId: 'other-board', canId: 42 },
    { deviceId: 'mock-board', canId: 7 },
    { deviceId: 'mock-board', canId: 99 },
  ]) {
    assertError(handle(backend, id++, 'device.selectController', params), 'invalid_state');
  }

  const before = profile(backend);
  const edited = result(
    handle(backend, id++, 'boardProfile.update', {
      savedDeviceId: 'saved-board',
      expectedRevision: before.revision,
      patch: { motor: 'hypercore_xr' },
    })
  ) as { revision: number };
  assert.equal(edited.revision, before.revision + 1);
  assert.deepEqual(
    result(
      handle(backend, id++, 'device.selectController', {
        deviceId: 'mock-board',
        canId: 42,
      })
    ),
    { accepted: true }
  );
  const selected = profile(backend);
  assert.equal(selected.revision, before.revision + 2);
  assert.equal(selected.values.motor, 'hypercore_xr');
  assert.equal(selected.values.preferredCanId, 42);
  assertError(
    handle(backend, id++, 'device.selectController', {
      deviceId: 'mock-board',
      canId: 42,
    }),
    'invalid_state'
  );
});

test('failed retry, replacement interruption, and frontend recovery remain authoritative', () => {
  const issueCodes: ConnectionIssueCode[] = [
    'transport_setup_failed',
    'identification_timeout',
    'no_controller_found',
    'too_many_can_devices',
    'too_many_controllers',
    'preferred_controller_unavailable',
    'controller_changed',
    'unsupported_controller',
    'unsupported_refloat',
  ];
  for (const code of issueCodes) {
    const backend = connectedBackend();
    backend.simulateConnectionFailure(code, 'x'.repeat(2_000));
    const failed = state(backend);
    assertState(failed);
    assert.equal(failed.activeConnection?.connectionIssue?.code, code);
    assert.equal(failed.activeConnection?.connectionIssue?.message.length, 1_024);
    if (code === 'controller_changed') {
      const sessions = result(handle(backend, 3_000, 'session.list', { savedDeviceId: 'saved-board', limit: 100 })) as {
        items: Array<{ state: string }>;
      };
      assert.ok(sessions.items.some((item) => item.state === 'interrupted'));
      assert.equal(failed.activeConnection?.activeSession, null);
    }
  }

  const retry = connectedBackend();
  retry.simulateConnectionFailure('preferred_controller_unavailable', 'Configured controller unavailable');
  assert.deepEqual(result(handle(retry, 20, 'device.connect', { deviceId: 'mock-board', trackingMode: 'saved' })), {
    accepted: true,
  });
  const recovered = state(retry);
  assertState(recovered);
  assert.equal(recovered.activeConnection?.state, 'ready');
  assert.equal(recovered.activeConnection?.connectionIssue, null);

  const selection = connectedBackend();
  selection.simulateControllerSelection(controllerCandidates);
  const beforeRecreation = state(selection);
  const afterRecreation = state(selection);
  assert.deepEqual(
    afterRecreation.activeConnection?.controllerCandidates,
    beforeRecreation.activeConnection?.controllerCandidates
  );
  assert.equal(afterRecreation.activeConnection?.state, 'selecting_controller');
});

const controllerCandidates: ControllerCandidate[] = [
  {
    canId: 42,
    hardwareName: 'THOR300',
    firmwareName: 'Refloat',
    vescVersion: '6.06',
    vescSupported: true,
  },
  {
    canId: 7,
    hardwareName: 'Unsupported VESC',
    firmwareName: null,
    vescVersion: '5.03',
    vescSupported: false,
  },
];

function connectedBackend() {
  const backend = new MockBackend();
  handle(backend, 1, 'bluetooth.scan', {});
  handle(backend, 2, 'device.connect', { deviceId: 'mock-board', trackingMode: 'saved' });
  return backend;
}

function simulator(name: SoftwareConnectionScenarioName) {
  return new VescSimulator(softwareConnectionScenarios[name].board);
}

function request(simulator: VescSimulator, bytes: number[]) {
  const values = simulator.receive(encodeFrame(Uint8Array.from(bytes)));
  assert.equal(values.length, 1);
  return values[0]!;
}

function payload(value: { frame: Uint8Array }) {
  const frame = value.frame;
  const short = frame[0] === 2;
  assert.ok(short);
  const length = frame[1]!;
  return [...frame.slice(2, 2 + length)];
}

function handle(backend: MockBackend, id: number, method: string, params: Record<string, unknown>) {
  return backend.handle({ id, method, params });
}

function state(backend: MockBackend): AppState {
  return result(handle(backend, 1_000, 'state.get', {})) as AppState;
}

function profile(backend: MockBackend) {
  return result(
    handle(backend, 1_001, 'boardProfile.get', {
      savedDeviceId: 'saved-board',
    })
  ) as {
    revision: number;
    values: Record<string, unknown>;
  };
}

function result(response: Response): unknown {
  assert.ok('result' in response, JSON.stringify(response));
  return response.result;
}

function assertError(response: Response, code: string) {
  assert.ok('error' in response, JSON.stringify(response));
  assert.equal(response.error.code, code);
}
