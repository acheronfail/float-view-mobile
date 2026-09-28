import { createServer } from 'node:http';
import type { Socket } from 'node:net';
import { FixtureRun, LIMITS, type FixtureCommand } from './session.ts';

export function validateRelayEndpoint(endpoint: string) {
  const url = new URL(endpoint);
  if (
    url.protocol !== 'http:' ||
    url.hostname !== '127.0.0.1' ||
    !url.port ||
    url.pathname !== '/' ||
    url.username ||
    url.password ||
    url.search ||
    url.hash
  )
    throw new Error('Fixture endpoint must be numeric loopback HTTP');
  return url;
}

export async function startFixtureRelay() {
  const run = new FixtureRun();
  let closed = false;
  const session = run.create();
  const sockets = new Set<Socket>();
  const active = new Map<string, number>();
  const buffered = new Map<string, number>();
  const polls = new Set<string>();
  const server = createServer(async (request, response) => {
    response.setHeader('Content-Type', 'application/json');
    const fail = (status: number) => {
      if (!response.writableEnded) response.writeHead(status).end('{"error":"Fixture relay rejected request"}');
    };
    if (
      request.socket.remoteAddress !== '127.0.0.1' ||
      request.method !== 'POST' ||
      request.url !== '/' ||
      request.headers.authorization !== `Bearer ${run.token}` ||
      request.headers.origin ||
      request.headers.host !== `127.0.0.1:${(server.address() as { port: number }).port}`
    ) {
      fail(403);
      return;
    }
    let dispatched = false;
    let admitted = false,
      poll = false;
    const sessionId = request.headers['x-fixture-session'];
    // Correlation header allows bounded admission before reading an encoded body.
    if (typeof sessionId !== 'string' || !run.has(sessionId)) {
      fail(403);
      return;
    }
    const cleanup = request.headers['x-fixture-close'] === '1';
    if (!cleanup && (active.get(sessionId) ?? 0) >= LIMITS.commands) {
      run.fail(sessionId);
      fail(429);
      return;
    }
    if (!cleanup) {
      active.set(sessionId, (active.get(sessionId) ?? 0) + 1);
      admitted = true;
    }
    let retained = 0;
    const timer = setTimeout(() => {
      run.fail(sessionId);
      fail(408);
      request.destroy();
    }, 4_500);
    try {
      const length = Number(request.headers['content-length']);
      const maximum = cleanup ? 1024 : LIMITS.message;
      if (!Number.isSafeInteger(length) || length < 0 || length > maximum) throw new Error('Message bound');
      const chunks: Buffer[] = [];
      let bytes = 0;
      for await (const chunk of request) {
        bytes += chunk.length;
        if (bytes > maximum || (!cleanup && (buffered.get(sessionId) ?? 0) + chunk.length > LIMITS.bytes))
          throw new Error('Message/session buffer bound');
        if (!cleanup) {
          retained += chunk.length;
          buffered.set(sessionId, (buffered.get(sessionId) ?? 0) + chunk.length);
        }
        chunks.push(chunk);
      }
      const input = JSON.parse(Buffer.concat(chunks, bytes).toString()) as FixtureCommand;
      if (input.session !== sessionId || (cleanup && input.command !== 'close')) throw new Error('Correlation');
      if (input.command === 'poll') {
        if (polls.has(sessionId)) throw new Error('Poll already outstanding');
        polls.add(sessionId);
        poll = true;
      }
      dispatched = true;
      const result = run.execute(input);
      response.end(JSON.stringify(result));
    } catch {
      if (!dispatched) run.fail(sessionId);
      fail(400);
    } finally {
      clearTimeout(timer);
      if (admitted) {
        const remaining = (active.get(sessionId) ?? 1) - 1;
        if (remaining) active.set(sessionId, remaining);
        else active.delete(sessionId);
      }
      const remainingBytes = (buffered.get(sessionId) ?? 0) - retained;
      if (remainingBytes) buffered.set(sessionId, remainingBytes);
      else buffered.delete(sessionId);
      if (poll) polls.delete(sessionId);
    }
  });
  server.maxConnections = LIMITS.commands + 2; // Reserved cleanup capacity.
  server.headersTimeout = 5_000;
  server.requestTimeout = 5_000;
  server.on('connection', (socket) => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
  });
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const endpoint = `http://127.0.0.1:${(server.address() as { port: number }).port}/`;
  return {
    endpoint,
    run: run.id,
    token: run.token,
    session: session.id,
    createSession() {
      return run.create().id;
    },
    async close() {
      if (closed) return;
      closed = true;
      run.close();
      for (const socket of sockets) socket.destroy();
      await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
    },
  };
}
export type FixtureRelay = Awaited<ReturnType<typeof startFixtureRelay>>;
