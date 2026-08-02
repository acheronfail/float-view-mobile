import assert from 'node:assert/strict';
import test from 'node:test';
import { ConformanceClient } from '../src/client.ts';
import { mandatoryMethods, runConformance } from '../src/suite.ts';
import { mockTarget } from '../targets/mock.ts';

test('mandatory contract has one concise method set', () => {
  assert.equal(mandatoryMethods.length, 18);
  assert.equal(new Set(mandatoryMethods).size, mandatoryMethods.length);
});

test('mock passes canonical black-box conformance', async () => {
  const result = await runConformance(mockTarget);
  assert.equal(result.target, 'mock');
});

test('state predicates are bounded and rejected when the socket closes', async () => {
  const endpoint = await mockTarget.start();
  const client = await ConformanceClient.open(endpoint.url, endpoint.origin);
  try {
    await client.request('state.get', {});
    await assert.rejects(
      client.waitForState(() => false, { timeoutMs: 10 }),
      /Timed out waiting for authoritative state/
    );
    const pending = client.waitForState(() => false, { timeoutMs: 1_000 });
    client.close();
    await assert.rejects(pending, /Backend closed the socket/);
  } finally {
    await mockTarget.stop();
  }
});
