import { afterEach, describe, expect, it } from 'vitest';
import { createServer, type ViteDevServer } from 'vite';
import WebSocket from 'ws';
import {
  DEVELOPMENT_ORIGIN,
  MOCK_BACKEND_PORT,
  getMockBootstrap,
  startMockServer,
  stopMockServer,
} from '../../../backend-mock';
import { backendProxy, MOCK_BACKEND_ORIGIN } from '../../../vite.config';

let running = false;
let vite: ViteDevServer | undefined;

afterEach(async () => {
  await vite?.close();
  vite = undefined;
  if (running) await stopMockServer();
  running = false;
});

describe.sequential('mock transport authentication', () => {
  it('rejects an invalid launch token through the Vite proxy', async () => {
    const port = await startMockServer(MOCK_BACKEND_PORT);
    running = true;
    vite = await createServer({
      configFile: false,
      logLevel: 'silent',
      server: {
        host: '127.0.0.1',
        proxy: { '/api': backendProxy(MOCK_BACKEND_ORIGIN) },
      },
    });
    await vite.listen(0);
    const address = vite.httpServer?.address();
    if (!address || typeof address === 'string') throw new Error('Vite test server did not bind TCP');
    const endpoint = new URL(getMockBootstrap(port).webSocketUrl);
    endpoint.port = String(address.port);
    endpoint.searchParams.set('token', 'invalid');
    const result = await new Promise<number>((resolve, reject) => {
      const socket = new WebSocket(endpoint, { origin: DEVELOPMENT_ORIGIN });
      socket.once('unexpected-response', (_request, response) => resolve(response.statusCode ?? 0));
      socket.once('error', reject);
    });
    expect(result).toBe(403);
  });

  it('authenticates bounded session-data POSTs before reading the body', async () => {
    const port = await startMockServer(MOCK_BACKEND_PORT);
    running = true;
    const bootstrap = getMockBootstrap(port);
    const token = new URL(bootstrap.webSocketUrl).searchParams.get('token');
    const request = {
      kind: 'series',
      sessionId: 'session-1',
      snapshot: {
        historyRevision: 0,
        committedSampleSequence: 1,
        committedEventSequence: 0,
        committedElapsedMs: 60_000,
      },
      range: { startElapsedMs: 0, endElapsedMs: 60_000 },
      fields: ['packVoltageV'],
      buckets: 2,
      gapPolicy: { thresholdSeconds: null },
    };
    const denied = await fetch(`http://127.0.0.1:${port}/api/session-data`, {
      method: 'POST',
      headers: { Origin: DEVELOPMENT_ORIGIN, 'X-Float-View-Token': 'invalid' },
      body: JSON.stringify(request),
    });
    expect(denied.status).toBe(403);
    const accepted = await fetch(`http://127.0.0.1:${port}/api/session-data`, {
      method: 'POST',
      headers: { Origin: DEVELOPMENT_ORIGIN, 'X-Float-View-Token': token ?? '', 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
    expect(accepted.status).toBe(200);
    expect((await accepted.json()).kind).toBe('series');
    expect(accepted.headers.get('cache-control')).toBe('no-store');
  });

  it('serves gzip event pages with no-store disclosure', async () => {
    const port = await startMockServer(0);
    running = true;
    const bootstrap = getMockBootstrap(port);
    const token = new URL(bootstrap.webSocketUrl).searchParams.get('token') ?? '';
    const base = {
      sessionId: 'session-1',
      snapshot: {
        historyRevision: 0,
        committedSampleSequence: 1,
        committedEventSequence: 2,
        committedElapsedMs: 60_000,
      },
      range: { startElapsedMs: 0, endElapsedMs: 60_000 },
    };
    const response = await fetch(`http://127.0.0.1:${port}/api/session-data`, {
      method: 'POST',
      headers: {
        Origin: DEVELOPMENT_ORIGIN,
        'X-Float-View-Token': token,
        'Content-Type': 'application/json',
        'Accept-Encoding': 'gzip',
      },
      body: JSON.stringify({ ...base, kind: 'events', limit: 1 }),
    });
    expect(response.status).toBe(200);
    expect(response.headers.get('content-encoding')).toBe('gzip');
    expect((await response.json()).page.sequence).toEqual([1]);
  });
});
