import { randomBytes } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import express from 'express';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path, { join } from 'node:path';
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
  nearestSample,
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
    const sessionId = activeMockBackend.importSession(Buffer.concat(chunks));
    response.type('application/json').json({ sessionId });
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
      const exported = createMockExport(sessionId, format);
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
    if (request.mode === 'nearest' && request.targetElapsedMs !== undefined) {
      const nearest = nearestSample(source, request.targetElapsedMs);
      rows = nearest ? [nearest] : [];
    } else {
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
  format: 'float-view-archive' | 'csv'
): { fileName: string; contentType: string; body: Buffer } {
  const fileName = `float-view-${sessionId}.${format === 'csv' ? 'csv' : 'floatview'}`;
  if (format === 'csv') {
    return {
      fileName,
      contentType: 'text/csv',
      body: Buffer.from('sequence,elapsed_ms,speed_kph,distance_m\n0,0,0,0\n1,60000,20,300\n', 'utf8'),
    };
  }

  const directory = mkdtempSync(join(tmpdir(), 'float-view-export-'));
  const pathName = join(directory, fileName);
  let database: DatabaseSync | undefined;
  try {
    database = new DatabaseSync(pathName);
    const escapedSessionId = sessionId.replaceAll("'", "''");
    database.exec(
      `CREATE TABLE float_view_archive_metadata(key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);\n` +
        `INSERT INTO float_view_archive_metadata(key, value) VALUES ('format_version', '1');\n` +
        `INSERT INTO float_view_archive_metadata(key, value) VALUES ('session', '${escapedSessionId}');`
    );
    database.close();
    database = undefined;
    return { fileName, contentType: 'application/octet-stream', body: readFileSync(pathName) };
  } finally {
    database?.close();
    rmSync(directory, { recursive: true, force: true });
  }
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
