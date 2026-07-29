import { afterEach, describe, expect, it } from 'vitest';
import { createServer, type ViteDevServer } from 'vite';
import WebSocket from 'ws';
import {
	DEVELOPMENT_ORIGIN,
	MOCK_BACKEND_PORT,
	getMockBootstrap,
	startMockServer,
	stopMockServer
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

describe('mock transport authentication', () => {
	it('rejects an invalid launch token through the Vite proxy', async () => {
		const port = await startMockServer(MOCK_BACKEND_PORT);
		running = true;
		vite = await createServer({
			configFile: false,
			logLevel: 'silent',
			server: {
				host: '127.0.0.1',
				proxy: { '/api': backendProxy(MOCK_BACKEND_ORIGIN) }
			}
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
});
