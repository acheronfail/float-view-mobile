import { randomBytes } from 'node:crypto';
import express from 'express';
import { createServer } from 'node:http';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { WebSocketServer } from 'ws';
import { MockBackend } from './api-v1.ts';

export const MOCK_BACKEND_PORT = 11003;
export const DEVELOPMENT_ORIGIN = 'http://127.0.0.1:11002';
const DEFAULT_PORT = MOCK_BACKEND_PORT;
const expectedOrigin = DEVELOPMENT_ORIGIN;
let sessionToken = randomBytes(32).toString('base64url');
let mockRunning = false;

const app = express();
const server = createServer(app);
let wss = createWebSocketServer();

app.get('/api/bootstrap', (_request, response) => {
	response.set('Cache-Control', 'no-store').json({
		webSocketUrl: '/api/ws',
		sessionToken
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
	const value = new WebSocketServer({ noServer: true, maxPayload: 65_536 });
	const backend = new MockBackend((event) => {
		const payload = JSON.stringify(event);
		for (const client of value.clients) {
			if (client.readyState === client.OPEN) client.send(payload);
		}
	});
	value.on('connection', (webSocket) => {
		webSocket.on('message', (data) => {
			webSocket.send(JSON.stringify(backend.handlePayload(data.toString())));
		});
	});
	return value;
}

export function getMockBootstrap(port: number) {
	return {
		webSocketUrl: `ws://127.0.0.1:${port}/api/ws?token=${encodeURIComponent(sessionToken)}`,
		origin: expectedOrigin
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

const isMain =
	!!process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (isMain) {
	startMockServer().then((port) => console.log(`Mock server started on ${port}`));
}
