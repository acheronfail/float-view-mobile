import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import { request } from 'node:http';
import WebSocket from 'ws';
import { ConformanceClient } from './client.ts';

/** Shared assertions for production HTTP/WS admission and multi-client state. */
export async function exerciseTransport(
  endpoint: { url: string; origin: string },
  afterPresentation?: () => Promise<void>
) {
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
    const initial = states[0];
    assert.ok(initial);
    assert.ok(
      states.every((state) => state.revision === initial.revision),
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
    const board = state.savedDevices.find((value) => value.deviceHistoryId === original.deviceHistoryId)!;
    await exerciseBoardExports(endpoint, archiveClient, board.savedDeviceId, afterPresentation);
    const csvExport = await fetch(endpoint.origin + '/api/session-export', {
      method: 'POST',
      headers,
      body: JSON.stringify({ sessionId: original.sessionId, format: 'csv' }),
    });
    assert.equal(csvExport.status, 200);
    assert.equal(csvExport.headers.get('content-type'), 'application/zip');
    assert.match(csvExport.headers.get('content-disposition')!, /\.csv\.zip"$/);
    const csvName = csvExport.headers.get('content-disposition')!.match(/filename="([^"]+)\.zip"/)![1]!;
    const temporary = mkdtempSync(join(tmpdir(), 'float-view-csv-'));
    try {
      const path = join(temporary, 'ride.csv.zip');
      writeFileSync(path, new Uint8Array(await csvExport.arrayBuffer()));
      assert.equal(execFileSync('unzip', ['-Z1', path], { encoding: 'utf8' }).trim(), csvName);
      execFileSync('unzip', ['-t', path]);
      const csv = execFileSync('unzip', ['-p', path, csvName], {
        encoding: 'utf8',
        maxBuffer: 128 * 1024 * 1024,
      });
      assert.ok(csv.startsWith('sequence,observed_at_ms,slow_observed_at_ms,elapsed_ms,'));
      assert.ok(csv.trimEnd().split('\n').length > 1);
    } finally {
      rmSync(temporary, { recursive: true, force: true });
    }
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
  await Promise.all(
    [true, false].map(
      (autoPong) =>
        new Promise<void>((resolve, reject) => {
          const ws = new WebSocket(endpoint.url, { origin: endpoint.origin, autoPong });
          let pings = 0;
          let finished = false;
          const timer = setTimeout(() => {
            finished = true;
            ws.close();
            if (!autoPong) reject(new Error('Unresponsive WebSocket retained its connection beyond the idle deadline'));
            else if (pings < 2) reject(new Error('Idle WebSocket did not receive heartbeats'));
            else resolve();
          }, 32_000);
          ws.on('ping', () => pings++);
          ws.on('error', (error) => {
            clearTimeout(timer);
            reject(error);
          });
          ws.on('close', () => {
            clearTimeout(timer);
            if (!finished && autoPong) reject(new Error('Responsive idle WebSocket closed during BLE wait'));
            else if (!autoPong) resolve();
          });
        })
    )
  );
}

export async function exerciseBoardExports(
  endpoint: { url: string; origin: string },
  client: ConformanceClient,
  savedDeviceId: string,
  afterPresentation?: () => Promise<void>
) {
  const token = new URL(endpoint.url).searchParams.get('token')!;
  const exportUrl = new URL('/api/session-export', endpoint.url.replace(/^ws/, 'http')).href;
  const headers = { Origin: endpoint.origin, 'X-Float-View-Token': token };
  for (const format of ['float-view-archive', 'csv'] as const) {
    const operationId = `export-${format}-${Date.now()}`;
    const before = (await client.request('state.get', {})).revision;
    const result = await client.request('session.export', { savedDeviceId, format, operationId });
    const progress = await client.request('session.export.status', { operationId });
    assert.equal(progress.phase, 'ready');
    assert.equal(progress.completedSessions, progress.totalSessions);
    assert.ok(progress.totalSessions > 0);
    assert.equal((await client.request('session.export.cancel', { operationId })).phase, 'ready');
    // Export progress itself does not publish product state. Native OS lifecycle
    // may independently update state while presenting a picker.
    assert.ok((await client.request('state.get', {})).revision >= before);
    assert.ok(result.fileName.endsWith('.zip'));
    if (result.delivery === 'presented') await afterPresentation?.();
  }
  const boardSessions = await client.request('session.list', {
    savedDeviceId,
    limit: 100,
  });
  for (const format of ['float-view-archive', 'csv'] as const) {
    const response = await fetch(exportUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify({ savedDeviceId, format }),
    });
    assert.equal(response.status, 200);
    assert.equal(response.headers.get('content-type'), 'application/zip');
    const directory = mkdtempSync(join(tmpdir(), 'float-view-bundle-'));
    try {
      const path = join(directory, 'bundle.zip');
      writeFileSync(path, new Uint8Array(await response.arrayBuffer()));
      execFileSync('unzip', ['-t', path]);
      const names = execFileSync('unzip', ['-Z1', path], { encoding: 'utf8' }).trim().split('\n');
      assert.equal(names.length, boardSessions.items.filter((item) => item.state !== 'active').length);
      assert.equal(new Set(names).size, names.length);
      assert.ok(names.every((name) => name.endsWith(format === 'csv' ? '.csv.zip' : '.floatview')));
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  }
  for (const payload of [{ format: 'csv' }, { sessionId: 'any-session', savedDeviceId, format: 'csv' }]) {
    const response = await fetch(exportUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify(payload),
    });
    assert.equal(response.status, 400);
  }
}
