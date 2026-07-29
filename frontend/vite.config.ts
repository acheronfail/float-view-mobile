import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin, type ProxyOptions } from 'vite';
import { MOCK_BACKEND_PORT, startMockServer, stopMockServer } from './backend-mock';

export const DEVELOPMENT_PORT = 11002;
export const MOCK_BACKEND_ORIGIN = `http://127.0.0.1:${MOCK_BACKEND_PORT}`;

export function selectedBackendOrigin(environment: NodeJS.ProcessEnv = process.env): string {
	const value = environment.FLOAT_VIEW_DEV_BACKEND_ORIGIN ?? MOCK_BACKEND_ORIGIN;
	const url = new URL(value);
	if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || url.pathname !== '/') {
		throw new Error('FLOAT_VIEW_DEV_BACKEND_ORIGIN must be an http://127.0.0.1:<port> origin');
	}
	return url.origin;
}

export function backendProxy(target: string): ProxyOptions {
	return {
		target,
		ws: true,
		changeOrigin: true,
		rewriteWsOrigin: true,
		timeout: 5_000,
		proxyTimeout: 5_000,
		configure(proxy) {
			// http-proxy does not relay a non-101 response received by its WebSocket
			// client. Forward it explicitly so authentication failures stay bounded.
			proxy.on('proxyReqWs', (request, _incoming, socket) => {
				request.once('response', (response) => {
					const status = `HTTP/1.1 ${response.statusCode ?? 500} ${
						response.statusMessage ?? 'WebSocket rejected'
					}`;
					const headers: string[] = [];
					for (let index = 0; index < response.rawHeaders.length; index += 2) {
						headers.push(`${response.rawHeaders[index]}: ${response.rawHeaders[index + 1]}`);
					}
					socket.write(`${status}\r\n${headers.join('\r\n')}\r\n\r\n`);
					response.pipe(socket);
				});
			});
		}
	};
}

function mockBackend(target: string): Plugin {
	return {
		name: 'float-view-mock-backend',
		apply: 'serve',
		async configureServer(server) {
			if (process.env.VITEST || target !== MOCK_BACKEND_ORIGIN) return;
			await startMockServer(MOCK_BACKEND_PORT);
			server.httpServer?.once('close', () => void stopMockServer());
		}
	};
}

const backendOrigin = selectedBackendOrigin();

export default defineConfig({
	plugins: [mockBackend(backendOrigin), tailwindcss(), sveltekit()],

	build: {
		rollupOptions: {
			output: {
				hashCharacters: 'hex'
			}
		}
	},

	server: {
		host: '127.0.0.1',
		port: DEVELOPMENT_PORT,
		strictPort: true,
		proxy: {
			'/api': backendProxy(backendOrigin)
		}
	},

	test: {
		expect: { requireAssertions: true },
		environment: 'node',
		include: ['src/**/*.{test,spec}.{js,ts}']
	}
});
