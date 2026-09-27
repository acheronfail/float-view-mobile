import assert from 'node:assert/strict';
import { request } from 'node:http';
import WebSocket from 'ws';
import { ConformanceClient } from './client.ts';

/** Shared assertions for production HTTP/WS admission and multi-client state. */
export async function exerciseTransport(endpoint: { url: string; origin: string }) {
  const socketUrl = new URL(endpoint.url);
  const token = socketUrl.searchParams.get('token')!;
  const headers = { Origin: endpoint.origin, 'X-Float-View-Token': token };
  for (const path of ['/api/session-data', '/api/session-import', '/api/session-export']) {
    for (const auth of [{}, { ...headers, Origin: 'null' }, { ...headers, 'X-Float-View-Token': 'wrong' }]) {
      const response = await fetch(endpoint.origin + path, { method: 'POST', headers: auth, body: '{' });
      assert.equal(response.status, 403, `${path} must authenticate before decoding`);
      assert.equal(response.headers.get('cache-control'), 'no-store');
    }
  }
  assert.equal((await fetch(endpoint.origin + '/missing')).status, 404);
  const bootstrap = await (await fetch(endpoint.origin + '/api/bootstrap')).json();
  assert.equal(bootstrap.sessionToken, token);
  for (const length of [16384, 16385]) {
    const body = '{}'.padEnd(length, ' ');
    const status = await new Promise<number>((resolve, reject) => {
      const req = request(endpoint.origin + '/api/session-data', { method: 'POST', headers }, (res) => {
        res.resume();
        res.on('end', () => resolve(res.statusCode!));
      });
      req.on('error', reject);
      req.write(body.slice(0, 8000));
      req.end(body.slice(8000));
    });
    assert.equal(status, length === 16384 ? 400 : 413);
  }
  for (const origin of ['null', endpoint.origin + '/']) {
    await new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(endpoint.url, { origin });
      ws.once('unexpected-response', (_, res) => {
        assert.equal(res.statusCode, 403);
        res.resume();
        ws.terminate();
        resolve();
      });
      ws.once('open', () => {
        ws.close();
        reject(new Error('unauthorized WebSocket accepted'));
      });
      ws.on('error', () => {});
    });
  }
  const first = await ConformanceClient.open(endpoint.url, endpoint.origin);
  const second = await ConformanceClient.open(endpoint.url, endpoint.origin);
  try {
    const states = await Promise.all(
      Array.from({ length: 8 }, (_, i) => (i % 2 ? first : second).request('state.get', {}))
    );
    assert.ok(
      states.every((state) => state.revision === states[0].revision),
      'subscriber count changed the revision'
    );
    const left = await first.request('state.get', {});
    second.close();
    const right = await first.request('state.get', {});
    assert.equal(left.revision, right.revision, 'subscriber removal changed revision');
  } finally {
    first.close();
    second.close();
  }
  const archiveClient = await ConformanceClient.open(endpoint.url, endpoint.origin);
  try {
    const state = await archiveClient.request('state.get', {});
    const sessions = (
      await Promise.all(
        state.savedDevices.map((board) =>
          archiveClient.request('session.list', { savedDeviceId: board.savedDeviceId, limit: 100 })
        )
      )
    ).flatMap((page) => page.items);
    const original = sessions.sort((a, b) => b.sampleCount - a.sampleCount)[0];
    assert.ok(original, 'archive fixture requires a completed session');
    const exported = await fetch(endpoint.origin + '/api/session-export', {
      method: 'POST',
      headers,
      body: JSON.stringify({ sessionId: original.sessionId, format: 'float-view-archive' }),
    });
    assert.equal(exported.status, 200);
    assert.match(exported.headers.get('content-disposition')!, /^attachment; filename="/);
    const archive = new Uint8Array(await exported.arrayBuffer());
    assert.ok(archive.length > 0);
    if (original.sampleCount >= 15000)
      assert.ok(archive.length > 1048576, 'large archive must exceed the host envelope');
    for (let abandoned = 0; abandoned < 2; abandoned++) {
      await new Promise<void>((resolve) => {
        const req = request(endpoint.origin + '/api/session-import', { method: 'POST', headers });
        req.on('error', () => resolve());
        req.write(archive.slice(0, 65536));
        setTimeout(() => {
          req.destroy();
          resolve();
        }, 100);
      });
    }
    // Cleanup is delivered by the bounded native drain (maximum wait 250 ms).
    await new Promise((resolve) => setTimeout(resolve, 500));
    const imported = await fetch(endpoint.origin + '/api/session-import', { method: 'POST', headers, body: archive });
    assert.equal(imported.status, 200);
    const { sessionId } = await imported.json();
    const detail = await archiveClient.request('session.get', { sessionId });
    assert.equal(detail.summary.deviceHistoryId, original.deviceHistoryId);
    assert.equal(detail.summary.sampleCount, original.sampleCount);
    await archiveClient.request('session.delete', { sessionId });
    const malformed = await fetch(endpoint.origin + '/api/session-import', {
      method: 'POST',
      headers,
      body: 'invalid archive',
    });
    assert.equal(malformed.status, 400);
  } finally {
    archiveClient.close();
  }
  for (const length of [65536, 65537]) {
    await new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(endpoint.url, { origin: endpoint.origin });
      const timer = setTimeout(() => {
        ws.terminate();
        reject(new Error('fragmented frame did not terminate'));
      }, 5000);
      let replied = false;
      ws.on('open', () => {
        const body = JSON.stringify({ id: 73, method: 'state.get', params: {} }).padEnd(length, ' ');
        ws.send(body.slice(0, 32768), { fin: false });
        ws.send(body.slice(32768), { fin: true });
      });
      ws.on('message', (bytes) => {
        const value = JSON.parse(bytes.toString());
        if (value.id === 73) {
          replied = true;
          assert.ok(value.result);
          ws.close();
        }
      });
      ws.on('close', () => {
        clearTimeout(timer);
        assert.equal(replied, length === 65536);
        resolve();
      });
      ws.on('error', reject);
    });
  }
}
