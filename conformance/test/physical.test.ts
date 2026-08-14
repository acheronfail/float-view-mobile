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
