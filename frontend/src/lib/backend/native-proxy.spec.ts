import { createServer as createHttpServer } from 'node:http';
import { afterEach, expect, it } from 'vitest';
import { createServer, type ViteDevServer } from 'vite';
import WebSocket, { WebSocketServer } from 'ws';
import { backendProxy } from '../../../vite.config';

let vite: ViteDevServer | undefined;
afterEach(async () => {
  await vite?.close();
  vite = undefined;
});

it('uses the device origin for HTTP and WebSocket authentication through a different host port', async () => {
  const deviceOrigin = 'http://127.0.0.1:49213';
  const native = createHttpServer((request, response) => {
    response.writeHead(request.headers.origin === deviceOrigin ? 200 : 403);
    response.end();
  });
  const sockets = new WebSocketServer({ noServer: true });
  native.on('upgrade', (request, socket, head) => {
    if (request.headers.origin !== deviceOrigin) {
      socket.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n');
      return;
    }
    sockets.handleUpgrade(request, socket, head, (client) => client.close());
  });
  await new Promise<void>((resolve) => native.listen(0, '127.0.0.1', resolve));
  try {
    const address = native.address();
    if (!address || typeof address === 'string') throw new Error('Missing test listener');
    vite = await createServer({
      configFile: false,
      logLevel: 'silent',
      server: {
        host: '127.0.0.1',
        proxy: { '/api': backendProxy(`http://127.0.0.1:${address.port}`, 5000, deviceOrigin) },
      },
    });
    await vite.listen(0);
    const proxyAddress = vite.httpServer?.address();
    if (!proxyAddress || typeof proxyAddress === 'string') throw new Error('Missing proxy listener');
    const origin = `http://127.0.0.1:${proxyAddress.port}`;
    expect((await fetch(`${origin}/api/probe`, { headers: { Origin: origin } })).status).toBe(200);
    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(`ws://127.0.0.1:${proxyAddress.port}/api/ws`, { origin });
      socket.once('open', () => {
        socket.close();
        resolve();
      });
      socket.once('error', reject);
    });
  } finally {
    await vite?.close();
    vite = undefined;
    sockets.close();
    await new Promise<void>((resolve) => native.close(() => resolve()));
  }
});
