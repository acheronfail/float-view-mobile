import { describe, expect, it } from 'vitest';
import { ApiClient, ApiProtocolError } from './client';

type Listener = (event: { data?: unknown }) => void;

class FakeWebSocket {
	readyState = 1;
	readonly sent: string[] = [];
	private readonly listeners = new Map<string, Set<Listener>>();

	addEventListener(type: string, listener: Listener) {
		let listeners = this.listeners.get(type);
		if (!listeners) {
			listeners = new Set();
			this.listeners.set(type, listeners);
		}
		listeners.add(listener);
	}

	removeEventListener(type: string, listener: Listener) {
		this.listeners.get(type)?.delete(listener);
	}

	send(data: string) {
		this.sent.push(data);
	}

	close() {
		this.readyState = 3;
		this.emit('close', {});
	}

	receive(message: unknown) {
		this.emit('message', { data: JSON.stringify(message) });
	}

	private emit(type: string, event: { data?: unknown }) {
		for (const listener of this.listeners.get(type) ?? []) listener(event);
	}
}

function setup() {
	const socket = new FakeWebSocket();
	const client = new ApiClient(socket as unknown as WebSocket, 100);
	return { socket, client };
}

describe('ApiClient', () => {
	it('correlates a typed request with its response', async () => {
		const { socket, client } = setup();
		const response = client.request('system.getSnapshot', {});
		const request = JSON.parse(socket.sent[0]);

		expect(request).toEqual({
			jsonrpc: '2.0',
			id: 1,
			method: 'system.getSnapshot',
			params: {}
		});

		socket.receive({
			jsonrpc: '2.0',
			id: request.id,
			result: {
				revision: 3,
				bluetooth: {
					radio: 'powered_on',
					scanPermission: 'granted',
					connectPermission: 'granted'
				},
				knownDevices: [],
				activeConnection: null,
				telemetrySubscriptions: [],
				loggingJobs: []
			}
		});

		await expect(response).resolves.toMatchObject({ revision: 3 });
		client.close();
	});

	it('rejects a request with the stable API error', async () => {
		const { socket, client } = setup();
		const response = client.request('system.getSnapshot', {});
		const request = JSON.parse(socket.sent[0]);

		socket.receive({
			jsonrpc: '2.0',
			id: request.id,
			error: {
				code: -32000,
				message: 'Hello required',
				data: { kind: 'invalid_state', recoverable: true }
			}
		});

		await expect(response).rejects.toEqual(
			expect.objectContaining({
				name: 'ApiRequestError',
				code: -32000,
				message: 'Hello required'
			})
		);
		client.close();
	});

	it('routes and unsubscribes server notifications', () => {
		const { socket, client } = setup();
		const received: unknown[] = [];
		const unsubscribe = client.onNotification('bluetooth.stateChanged', (params) =>
			received.push(params)
		);

		socket.receive({
			jsonrpc: '2.0',
			method: 'bluetooth.stateChanged',
			params: { radio: 'powered_off' }
		});
		unsubscribe();
		socket.receive({
			jsonrpc: '2.0',
			method: 'bluetooth.stateChanged',
			params: { radio: 'powered_on' }
		});

		expect(received).toEqual([{ radio: 'powered_off' }]);
		client.close();
	});

	it('rejects pending requests when the socket closes', async () => {
		const { socket, client } = setup();
		const response = client.request('system.getSnapshot', {});
		socket.close();

		await expect(response).rejects.toBeInstanceOf(ApiProtocolError);
	});
});
