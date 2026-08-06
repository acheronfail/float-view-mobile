import assert from 'node:assert/strict';
import test from 'node:test';
import { SHUTDOWN_NO_OP_WAIT_MS } from '../src/physical.ts';

test('physical shutdown no-op wait exceeds the native shutdown-intent window', () => {
  assert.equal(SHUTDOWN_NO_OP_WAIT_MS, 35_000);
});
