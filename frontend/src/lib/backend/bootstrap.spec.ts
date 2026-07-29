import { describe, expect, it } from 'vitest';
import { bootstrapWebSocketUrl } from './bootstrap';

describe('bootstrap WebSocket endpoint', () => {
	it('routes native development sockets through the Vite origin', () => {
		expect(
			bootstrapWebSocketUrl(
				{
					webSocketUrl: 'ws://127.0.0.1:11010/api/ws?existing=yes',
					sessionToken: 'launch token'
				},
				'http://127.0.0.1:11002/',
				true
			).toString()
		).toBe('ws://127.0.0.1:11002/api/ws?existing=yes&token=launch+token');
	});

	it('preserves the backend endpoint in production', () => {
		expect(
			bootstrapWebSocketUrl(
				{ webSocketUrl: 'ws://127.0.0.1:11011/api/ws', sessionToken: 'token' },
				'http://127.0.0.1:11011/',
				false
			).toString()
		).toBe('ws://127.0.0.1:11011/api/ws?token=token');
	});
});
