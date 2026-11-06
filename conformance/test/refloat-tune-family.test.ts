import assert from 'node:assert/strict';
import test from 'node:test';
import { MockBackend } from '../../frontend/backend-mock/api-v1.ts';
import type { RefloatTune } from '../../frontend/src/lib/backend/contract.ts';

test('mock preserves the shared audited Tune family when loading existing beta records', () => {
  const versions = ['1.3.1-beta2', '1.3.1-beta3', '1.3.1'];
  const identity = {
    packageName: 'refloat' as const,
    packageVersion: '1.3.1',
    schemaFingerprint: 'a'.repeat(64),
    wireSignature: 42,
  };
  const tunes: RefloatTune[] = versions.map((version, index) => ({
    tuneId: `family-${index}`,
    revision: 1,
    name: version,
    description: null,
    identity: { ...identity, packageVersion: version },
    values: { gain: { type: 'integer', value: 2 } },
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  }));
  const backend = new MockBackend(() => {}, { initialRefloatTunes: tunes });
  let id = 0;
  const request = (method: string, params: object): any => {
    const response = backend.handle({ id: ++id, method, params });
    assert.ok('result' in response, JSON.stringify(response));
    return response.result;
  };
  let cursor = null;
  const ids = new Set<string>();
  for (const version of versions) {
    const page = request('refloat.tune.list', { identity: { ...identity, packageVersion: version }, limit: 1, cursor });
    assert.equal(page.items.length, 1);
    assert.equal(page.items[0].identity.packageVersion, '1.3.1');
    ids.add(page.items[0].tuneId);
    cursor = page.nextCursor;
  }
  assert.equal(ids.size, 3);
  assert.equal(cursor, null);
  assert.equal(request('refloat.tune.get', { tuneId: 'family-0' }).identity.packageVersion, '1.3.1');
  for (const version of ['1.3.1-beta1', '1.3.1-beta4', '1.3.2']) {
    assert.deepEqual(
      request('refloat.tune.list', { identity: { ...identity, packageVersion: version }, limit: 50 }).items,
      []
    );
  }
});
