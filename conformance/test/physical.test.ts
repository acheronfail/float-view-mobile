import assert from 'node:assert/strict';
import test from 'node:test';
import { shouldDeferPhysicalLocation, SHUTDOWN_NO_OP_WAIT_MS } from '../src/physical.ts';

test('physical shutdown no-op wait exceeds the native shutdown-intent window', () => {
  assert.equal(SHUTDOWN_NO_OP_WAIT_MS, 35_000);
});

test('physical location deferral requires an explicit opt-in', () => {
  assert.equal(shouldDeferPhysicalLocation(undefined), false);
  assert.equal(shouldDeferPhysicalLocation('0'), false);
  assert.equal(shouldDeferPhysicalLocation('1'), true);
});

test('1.3.0 physical selection covers lifecycle routes and preserves legacy regression', async () => {
  const { physicalScenario, fixtureScenarios } = await import('../fixture/scenarios.ts');
  for (const name of ['direct', 'ride', 'unexpected-disconnect', 'background', 'control-can'] as const) {
    const selected = physicalScenario(name, '1.3.0');
    const controller = selected.board.controllers[0]!;
    assert.equal(controller.refloatVersion, '1.3.0');
    assert.equal(controller.refloat?.version, '1.3.0');
    assert.deepEqual([...controller.refloatInfoPayload.slice(25, 28)], [1, 3, 0]);
    assert.equal(controller.telemetry?.phase(), name === 'control-can' ? 'idle' : 'riding');
  }
  assert.equal(physicalScenario('ride'), fixtureScenarios.ride);
  assert.equal(physicalScenario('direct-legacy', '1.3.0'), fixtureScenarios['direct-legacy']);
  assert.throws(() => physicalScenario('ride', '1.3.1'), /exactly 1.3.0/);
});
