import { randomBytes } from 'node:crypto';
import express from 'express';
import { createServer } from 'node:http';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { WebSocketServer } from 'ws';
import {
  MAX_MESSAGE_BYTES,
  MAX_SESSION_DATA_REQUEST_BYTES,
  MAX_SESSION_IMPORT_BYTES,
} from '../src/lib/backend/contract.ts';
import { MockBackend } from './api-v1.ts';
import { validateSessionDataRequest, type SessionDataRequest } from '../src/lib/backend/session-data.ts';
import {
  decodeCursor,
  encodeCursor,
  paginateEvents,
  paginateSamples,
  reduceSeries,
  type ReferenceEvent,
  type ReferenceSample,
} from '../src/lib/backend/session-reference.ts';
import { gzip } from 'node:zlib';
import { promisify } from 'node:util';

export const MOCK_BACKEND_PORT = 11003;
export const DEVELOPMENT_ORIGIN = 'http://127.0.0.1:11002';
const DEFAULT_PORT = MOCK_BACKEND_PORT;
const expectedOrigin = DEVELOPMENT_ORIGIN;
const gzipAsync = promisify(gzip);
let sessionToken = randomBytes(32).toString('base64url');
let mockRunning = false;
let activeMockBackend: MockBackend | undefined;

const app = express();
const server = createServer(app);
let wss = createWebSocketServer();

app.get('/api/bootstrap', (_request, response) => {
  response.set('Cache-Control', 'no-store').json({
    webSocketUrl: '/api/ws',
    sessionDataUrl: '/api/session-data',
    sessionExportUrl: '/api/session-export',
    sessionImportUrl: '/api/session-import',
    sessionToken,
    platform: 'web',
  });
});

app.post('/api/session-import', (request, response) => {
  response.set('Cache-Control', 'no-store');
  if (request.headers.origin !== expectedOrigin || request.headers['x-float-view-token'] !== sessionToken) {
    response.status(403).end();
    return;
  }
  if (Number(request.headers['content-length'] ?? 0) > MAX_SESSION_IMPORT_BYTES) {
    response.status(413).end();
    return;
  }
  const chunks: Buffer[] = [];
  let bytes = 0;
  request.on('data', (chunk: Buffer) => {
    bytes += chunk.byteLength;
    if (bytes <= MAX_SESSION_IMPORT_BYTES) chunks.push(chunk);
  });
  request.on('end', () => {
    if (bytes > MAX_SESSION_IMPORT_BYTES) {
      response.status(413).end();
      return;
    }
    if (!activeMockBackend) {
      response.status(503).end();
      return;
    }
    try {
      const sessionId = activeMockBackend.importSession(Buffer.concat(chunks));
      response.type('application/json').json({ sessionId });
    } catch {
      response.status(400).end();
    }
  });
});

app.post('/api/session-export', (request, response) => {
  response.set('Cache-Control', 'no-store');
  if (request.headers.origin !== expectedOrigin || request.headers['x-float-view-token'] !== sessionToken) {
    response.status(403).end();
    return;
  }
  if (Number(request.headers['content-length'] ?? 0) > 16_384) {
    response.status(413).end();
    return;
  }
  const chunks: Buffer[] = [];
  let bytes = 0;
  request.on('data', (chunk: Buffer) => {
    bytes += chunk.byteLength;
    if (bytes <= 16_384) chunks.push(chunk);
  });
  request.on('end', () => {
    if (bytes > 16_384) {
      response.status(413).end();
      return;
    }
    try {
      const payload = JSON.parse(Buffer.concat(chunks).toString('utf8')) as { sessionId?: unknown; format?: unknown };
      const sessionId = typeof payload.sessionId === 'string' ? payload.sessionId : '';
      const format = payload.format === 'csv' || payload.format === 'float-view-archive' ? payload.format : undefined;
      if (!sessionId || !format) {
        response.status(400).end();
        return;
      }
      if (!activeMockBackend?.hasSession(sessionId)) {
        response.status(404).end();
        return;
      }
      const exported = createMockExport(sessionId, format, activeMockBackend.archiveIdentity(sessionId));
      response.set({
        'Content-Type': exported.contentType,
        'Content-Disposition': `attachment; filename="${exported.fileName}"`,
      });
      response.status(200).end(exported.body);
    } catch {
      response.status(400).end();
    }
  });
});

app.post('/api/session-data', (request, response) => {
  response.set('Cache-Control', 'no-store');
  if (request.headers.origin !== expectedOrigin || request.headers['x-float-view-token'] !== sessionToken) {
    response.status(403).end();
    return;
  }
  let bytes = 0;
  const chunks: Buffer[] = [];
  request.on('data', (chunk: Buffer) => {
    bytes += chunk.byteLength;
    if (bytes > MAX_SESSION_DATA_REQUEST_BYTES) {
      request.destroy();
      response.status(413).end();
      return;
    }
    chunks.push(chunk);
  });
  request.on('end', async () => {
    try {
      const requestValue = validateSessionDataRequest(JSON.parse(Buffer.concat(chunks).toString('utf8')));
      const body = JSON.stringify(mockSessionDataResult(requestValue));
      if (request.headers['accept-encoding']?.includes('gzip')) {
        response.set('Content-Encoding', 'gzip');
        response.set('Content-Type', 'application/json');
        response.end(await gzipAsync(Buffer.from(body, 'utf8')));
      } else {
        response.type('application/json').send(body);
      }
    } catch {
      response.status(400).json({ error: { code: 'invalid_request', message: 'Invalid session-data request' } });
    }
  });
});

server.on('upgrade', (request, socket, head) => {
  const url = new URL(request.url ?? '/', 'http://127.0.0.1');
  const origin = request.headers.origin;
  if (
    url.pathname !== '/api/ws' ||
    url.searchParams.get('token') !== sessionToken ||
    (origin !== expectedOrigin && origin !== undefined)
  ) {
    socket.write('HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n');
    socket.destroy();
    return;
  }

  wss.handleUpgrade(request, socket, head, (webSocket) => {
    wss.emit('connection', webSocket, request);
  });
});

export function startMockServer(port = DEFAULT_PORT): Promise<number> {
  if (mockRunning) return Promise.reject(new Error('Mock server is already running'));
  sessionToken = randomBytes(32).toString('base64url');
  wss = createWebSocketServer();
  return new Promise((resolve, reject) => {
    const handleError = (error: Error) => reject(error);
    server.once('error', handleError);
    server.listen(port, '127.0.0.1', () => {
      server.removeListener('error', handleError);
      const address = server.address();
      if (!address || typeof address === 'string') {
        reject(new Error('Mock server did not bind a TCP port'));
        return;
      }
      mockRunning = true;
      resolve(address.port);
    });
  });
}

function createWebSocketServer() {
  const value = new WebSocketServer({ noServer: true, maxPayload: MAX_MESSAGE_BYTES });
  const delivery = new WeakMap<import('ws').WebSocket, { sendingState: boolean; pendingState?: string }>();
  function sendNewestState(client: import('ws').WebSocket, payload: string) {
    const state = delivery.get(client) ?? { sendingState: false };
    delivery.set(client, state);
    if (state.sendingState) {
      state.pendingState = payload;
      return;
    }
    state.sendingState = true;
    client.send(payload, () => {
      state.sendingState = false;
      const pending = state.pendingState;
      state.pendingState = undefined;
      if (pending && client.readyState === client.OPEN) sendNewestState(client, pending);
    });
  }
  const backend = new MockBackend((event) => {
    const payload = JSON.stringify(event);
    for (const client of value.clients) {
      if (client.readyState === client.OPEN) sendNewestState(client, payload);
    }
  });
  activeMockBackend = backend;
  let telemetryTimer: NodeJS.Timeout;
  const publishNextSample = () => {
    backend.publishTelemetry();
    telemetryTimer = setTimeout(publishNextSample, backend.telemetryCadenceMs);
    telemetryTimer.unref();
  };
  publishNextSample();
  value.once('close', () => {
    clearTimeout(telemetryTimer);
    if (activeMockBackend === backend) activeMockBackend = undefined;
  });
  value.on('connection', (webSocket) => {
    webSocket.on('message', (data) => {
      webSocket.send(JSON.stringify(backend.handlePayload(data.toString())));
    });
  });
  return value;
}

function mockSessionDataResult(request: SessionDataRequest): Record<string, unknown> {
  if (request.snapshot.historyRevision !== 0) {
    throw new Error('Published session history revision is stale');
  }
  const source: ReferenceSample[] = [
    { elapsedMs: 0, sequence: 0, values: { packVoltageV: 60, speedKph: 0 } },
    { elapsedMs: 60_000, sequence: 1, values: { packVoltageV: 55, speedKph: 20 } },
  ].filter((row) => row.elapsedMs >= request.range.startElapsedMs && row.elapsedMs <= request.range.endElapsedMs);
  if (request.kind === 'series') {
    return {
      kind: 'series',
      sessionId: request.sessionId,
      snapshot: request.snapshot,
      range: request.range,
      gapPolicy: request.gapPolicy,
      fields: request.fields.map((name) => ({ name, source: 'stable' })),
      columns: reduceSeries(source, request.fields, request.range, request.buckets, request.gapPolicy),
    };
  }
  if (request.kind === 'samples') {
    let rows: ReferenceSample[];
    let nextCursor: string | null = null;
    const cursor = request.cursor ? decodeCursor(request.cursor) : null;
    if (
      cursor &&
      (cursor.sessionId !== request.sessionId ||
        JSON.stringify(cursor.snapshot) !== JSON.stringify(request.snapshot) ||
        JSON.stringify(cursor.range) !== JSON.stringify(request.range))
    ) {
      throw new Error('Cursor does not match the requested snapshot');
    }
    const page = paginateSamples(source, request.limit, cursor);
    rows = page.rows;
    if (rows.length && rows.length < source.length) {
      const last = rows.at(-1)!;
      nextCursor = encodeCursor({
        sessionId: request.sessionId,
        snapshot: request.snapshot,
        range: request.range,
        elapsedMs: last.elapsedMs,
        sequence: last.sequence,
      });
    }
    return {
      kind: 'samples',
      sessionId: request.sessionId,
      snapshot: request.snapshot,
      range: request.range,
      mode: request.mode,
      fields: request.fields.map((name) => ({ name, source: 'stable' })),
      elapsedMs: rows.map((row) => row.elapsedMs),
      sequence: rows.map((row) => row.sequence),
      values: Object.fromEntries(
        request.fields.map((field) => [field, rows.map((row) => row.values[field as keyof typeof row.values] ?? null)])
      ),
      nextCursor,
    };
  }
  if (request.kind === 'locations') {
    if (request.sessionId !== 'session-1') {
      return {
        kind: 'locations',
        sessionId: request.sessionId,
        snapshot: request.snapshot,
        range: request.range,
        points: [],
        gaps: [],
        nextCursor: null,
        locationBoundary: { lastSequence: 0, lastElapsedMs: 0 },
      };
    }
    const points = [
      {
        locationSequence: 1,
        elapsedMs: 10_000,
        latitude: -33.8688,
        longitude: 151.2093,
        accuracyM: 4.5,
        observedAt: '2026-01-01T00:00:10.000Z',
      },
      {
        locationSequence: 2,
        elapsedMs: 20_000,
        latitude: -33.869,
        longitude: 151.2098,
        accuracyM: null,
        observedAt: '2026-01-01T00:00:20.000Z',
      },
      {
        locationSequence: 3,
        elapsedMs: 50_000,
        latitude: -33.87,
        longitude: 151.2105,
        accuracyM: 12,
        observedAt: '2026-01-01T00:00:50.000Z',
      },
    ].filter(
      (point) => point.elapsedMs >= request.range.startElapsedMs && point.elapsedMs <= request.range.endElapsedMs
    );
    const gaps =
      request.range.startElapsedMs <= 20_000 && request.range.endElapsedMs >= 20_000
        ? [{ startElapsedMs: 20_001, endElapsedMs: 49_999, reason: 'noFix' as const }]
        : [];
    const cursor = request.cursor ? decodeCursor(request.cursor) : null;
    if (
      cursor &&
      (cursor.sessionId !== request.sessionId ||
        JSON.stringify(cursor.snapshot) !== JSON.stringify(request.snapshot) ||
        JSON.stringify(cursor.range) !== JSON.stringify(request.range))
    ) {
      throw new Error('Cursor does not match the requested snapshot');
    }
    const page = points.slice(cursor ? points.findIndex((point) => point.locationSequence > cursor.sequence) : 0);
    const rows = page.slice(0, request.limit);
    const last = rows.at(-1);
    return {
      kind: 'locations',
      sessionId: request.sessionId,
      snapshot: request.snapshot,
      range: request.range,
      points: rows,
      gaps,
      nextCursor:
        last && rows.length < page.length
          ? encodeCursor({
              sessionId: request.sessionId,
              snapshot: request.snapshot,
              range: request.range,
              elapsedMs: last.elapsedMs,
              sequence: last.locationSequence,
            })
          : null,
      locationBoundary: { lastSequence: 3, lastElapsedMs: 50_000 },
    };
  }
  const events: ReferenceEvent[] = [
    { sequence: 1, sourceSampleSequence: 0, elapsedMs: 0, kind: 'packageState', value: 'running' },
    { sequence: 2, sourceSampleSequence: 1, elapsedMs: 60_000, kind: 'packageState', value: 'stopped' },
  ].filter((event) => event.elapsedMs >= request.range.startElapsedMs && event.elapsedMs <= request.range.endElapsedMs);
  const cursor = request.cursor ? decodeCursor(request.cursor) : null;
  if (
    cursor &&
    (cursor.sessionId !== request.sessionId ||
      JSON.stringify(cursor.snapshot) !== JSON.stringify(request.snapshot) ||
      JSON.stringify(cursor.range) !== JSON.stringify(request.range))
  ) {
    throw new Error('Cursor does not match the requested snapshot');
  }
  const page = paginateEvents(events, request.limit, cursor);
  const last = page.rows.at(-1);
  return {
    kind: 'events',
    sessionId: request.sessionId,
    snapshot: request.snapshot,
    range: request.range,
    page: {
      elapsedMs: page.rows.map((event) => event.elapsedMs),
      sequence: page.rows.map((event) => event.sequence),
      sourceSampleSequence: page.rows.map((event) => event.sourceSampleSequence),
      kinds: page.rows.map((event) => event.kind),
      values: page.rows.map((event) => event.value),
    },
    nextCursor:
      last && page.rows.length < events.length
        ? encodeCursor({
            sessionId: request.sessionId,
            snapshot: request.snapshot,
            range: request.range,
            elapsedMs: last.elapsedMs,
            sequence: last.sequence,
          })
        : null,
  };
}

export function getMockBootstrap(port: number) {
  return {
    webSocketUrl: `ws://127.0.0.1:${port}/api/ws?token=${encodeURIComponent(sessionToken)}`,
    sessionExportUrl: '/api/session-export',
    sessionImportUrl: '/api/session-import',
    origin: expectedOrigin,
    platform: 'web' as const,
  };
}

function createMockExport(
  sessionId: string,
  format: 'float-view-archive' | 'csv',
  identity: { deviceHistoryId: string; displayName: string } | null
): { fileName: string; contentType: string; body: Buffer } {
  const fileName = `float-view-${sessionId}.${format === 'csv' ? 'csv' : 'floatview'}`;
  if (format === 'csv') {
    return {
      fileName,
      contentType: 'text/csv',
      body: Buffer.from('sequence,elapsed_ms,speed_kph,distance_m\n0,0,0,0\n1,60000,20,300\n', 'utf8'),
    };
  }

  if (!identity) throw new Error('Session was not found');
  return {
    fileName,
    contentType: 'application/octet-stream',
    body: Buffer.from(JSON.stringify({ sessionId, ...identity }), 'utf8'),
  };
}

export function stopMockServer(): Promise<void> {
  for (const client of wss.clients) client.terminate();
  return new Promise((resolve, reject) => {
    wss.close((webSocketError) => {
      server.close((serverError) => {
        const error = webSocketError ?? serverError;
        if (error) reject(error);
        else {
          mockRunning = false;
          resolve();
        }
      });
    });
  });
}

const isMain = !!process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (isMain) {
  startMockServer().then((port) => console.log(`Mock server started on ${port}`));
}
