import assert from 'node:assert/strict';
import test from 'node:test';
import config from '../fixture-dev/vite.config.ts';
import { FIXTURE_DEV_ORIGIN, FIXTURE_DEV_PORT } from '../fixture-dev/src/lib/contract.ts';

test('fixture development server owns one strict loopback port without production build config', () => {
  assert.equal(FIXTURE_DEV_PORT, 11004);
  assert.equal(FIXTURE_DEV_ORIGIN, 'http://127.0.0.1:11004');
  assert.equal(typeof config, 'object');
  if (typeof config !== 'object') throw new Error('Expected static Vite configuration');
  assert.equal(config.server?.host, '127.0.0.1');
  assert.equal(config.server?.port, 11004);
  assert.equal(config.server?.strictPort, true);
  assert.equal(config.build, undefined);
});
