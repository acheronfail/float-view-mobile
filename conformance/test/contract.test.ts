import assert from 'node:assert/strict';
import test from 'node:test';
import { ConformanceClient } from '../src/client.ts';
import type { AppState, DocumentOperationState } from '../../frontend/src/lib/backend/contract.ts';
import { assertState, mandatoryMethods, runConformance } from '../src/suite.ts';
import { mockTarget } from '../targets/mock.ts';

test('mandatory contract has one concise method set', () => {
  assert.equal(mandatoryMethods.length, 35);
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

test('document operation conformance accepts every terminal shape and rejects inconsistent state', async () => {
  const endpoint = await mockTarget.start();
  const client = await ConformanceClient.open(endpoint.url, endpoint.origin);
  try {
    const initial = await client.request('state.get', {});
    const operations: DocumentOperationState[] = [
      {
        operationId: 'document-1',
        kind: 'import',
        status: 'presenting',
        error: null,
        exportDisplayName: null,
        sessionId: null,
      },
      {
        operationId: 'document-1',
        kind: 'import',
        status: 'completed',
        error: null,
        exportDisplayName: null,
        sessionId: 'imported-session',
      },
      {
        operationId: 'document-2',
        kind: 'import',
        status: 'cancelled',
        error: null,
        exportDisplayName: null,
        sessionId: null,
      },
      {
        operationId: 'document-3',
        kind: 'export',
        status: 'failed',
        error: { code: 'unavailable', message: 'Document presentation is unavailable' },
        exportDisplayName: 'ride.csv',
        sessionId: null,
      },
    ];
    for (const operation of operations) {
      assert.doesNotThrow(() => assertState({ ...initial, documentOperation: operation }));
    }
    assert.throws(() =>
      assertState({
        ...initial,
        documentOperation: {
          ...operations[1]!,
          kind: 'export',
        },
      } as AppState)
    );
  } finally {
    client.close();
    await mockTarget.stop();
  }
});
