import { test } from 'node:test';
import assert from 'node:assert/strict';
import { SESSION_DATA_VECTORS } from '../../frontend/src/lib/backend/session-vectors.ts';
import { reduceSeries, selectPruneRows } from '../../frontend/src/lib/backend/session-reference.ts';
import { ConformanceClient } from '../src/client.ts';
import { getMockBootstrap, startMockServer, stopMockServer } from '../../frontend/backend-mock/index.ts';

test('canonical historical vectors are immutable and preserve M4 role order', () => {
  const vector = SESSION_DATA_VECTORS[0]!;
  assert.equal(Object.isFrozen(SESSION_DATA_VECTORS), true);
  const result = reduceSeries(vector.samples, vector.fields, vector.range, 4, vector.policies[0]!);
  assert.deepEqual(result.speedKph!.values, vector.expectedSegments.speedKph);
  assert.deepEqual(result.packVoltageV!.values, vector.expectedSegments.packVoltageV);
  assert.ok(result.extensionLate!.sequence.includes(12));
});

test('canonical pruning vectors retain protected source evidence at aligned resolutions', () => {
  const vector = SESSION_DATA_VECTORS[0]!;
  const eventSources = new Set(vector.events.map((event) => event.sourceSampleSequence));
  const retained = selectPruneRows(vector.samples, vector.fields, 5, new Set([12]), eventSources);
  assert.deepEqual(
    retained,
    [...retained].sort((left, right) => left - right)
  );
  assert.ok(retained.includes(10));
  assert.ok(retained.includes(18));
  assert.ok(retained.includes(12));
});

test('session-data conformance preserves cursor identity and rejects bounded/auth failures', async () => {
  const port = await startMockServer(0);
  const bootstrap = getMockBootstrap(port);
  const client = await ConformanceClient.open(bootstrap.webSocketUrl, bootstrap.origin);
  const snapshot = {
    historyRevision: 0,
    committedSampleSequence: 1,
    committedEventSequence: 2,
    committedElapsedMs: 60_000,
  };
  const range = { startElapsedMs: 0, endElapsedMs: 60_000 };
  try {
    const first = await client.sessionData({
      kind: 'samples', sessionId: 'session-1', snapshot, range,
      fields: ['speedKph'], mode: 'window', limit: 1,
    });
    assert.equal(first.kind, 'samples');
    assert.ok(first.nextCursor);
    const second = await client.sessionData({
      kind: 'samples', sessionId: 'session-1', snapshot, range,
      fields: ['speedKph'], mode: 'window', limit: 1, cursor: first.nextCursor,
    });
    assert.equal(second.kind, 'samples');
    if (second.kind !== 'samples') throw new Error('Expected sample page');
    assert.deepEqual(second.sequence, [1]);

    const eventPage = await client.sessionData({
      kind: 'events', sessionId: 'session-1', snapshot, range, limit: 1,
    });
    assert.equal(eventPage.kind, 'events');
    assert.ok(eventPage.nextCursor);
    const replay = await client.sessionData({
      kind: 'events', sessionId: 'session-1', snapshot, range,
      limit: 1, cursor: eventPage.nextCursor,
    });
    if (replay.kind !== 'events') throw new Error('Expected event page');
    assert.deepEqual(replay.page.sequence, [2]);

    await assert.rejects(
      client.sessionData({
        kind: 'samples', sessionId: 'session-1',
        snapshot: { ...snapshot, historyRevision: 1 }, range,
        fields: ['speedKph'], mode: 'window', limit: 1, cursor: first.nextCursor,
      }),
      /HTTP 400/
    );

    const webSocketUrl = new URL(bootstrap.webSocketUrl);
    const endpoint = new URL('/api/session-data', `${webSocketUrl.protocol === 'wss:' ? 'https:' : 'http:'}//${webSocketUrl.host}`);
    const token = webSocketUrl.searchParams.get('token');
    const forbidden = await fetch(endpoint, {
      method: 'POST', body: '{}', headers: { Origin: bootstrap.origin, 'X-Float-View-Token': `${token}-wrong` },
    });
    assert.equal(forbidden.status, 403);
    await assert.rejects(
      fetch(endpoint, {
        method: 'POST', body: JSON.stringify({ padding: 'x'.repeat(17_000) }),
        headers: { Origin: bootstrap.origin, 'X-Float-View-Token': token ?? '' },
      }),
      /fetch failed|other side closed/
    );
  } finally {
    client.close();
    await stopMockServer();
  }
});
