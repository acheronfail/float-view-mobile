import {
	BleConnectResult,
	BleScanResult,
	WsMessage,
	type ZWsMessage
} from './backend-client.types';
import { ResponseError } from './errors/response';

/**
 * Client to communicate with the iOS/Android backends
 */
class BackendClient {
	constructor() {}

	private async fetch<T>(
		input: RequestInfo | URL,
		init?: RequestInit,
		parse?: (x: unknown) => T
	): Promise<T> {
		const response = await fetch(input, init);
		if (!response.ok) {
			throw new ResponseError(response.status, await response.text());
		}

		const json = await response.json();
		return parse ? parse(json) : (json as T);
	}

	public async scanBleDevices() {
		return this.fetch('/api/ble/scan', undefined, BleScanResult.parse);
	}

	public async connectBleDevice(deviceId: string) {
		return this.fetch(
			`/api/ble/connect`,
			{
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ deviceId })
			},
			BleConnectResult.parse
		);
	}

	public connectWebsocket() {
		return new Promise<[WebSocket, Event]>((resolve, reject) => {
			const ws = new WebSocket(`ws://${window.location.host}/ws`);

			const errorListener = (event: Event) => reject(event);
			ws.addEventListener('error', errorListener);

			const openListener = (event: Event) => {
				resolve([ws, event]);
				ws.removeEventListener('error', errorListener);
				ws.removeEventListener('open', openListener);
			};
			ws.addEventListener('open', openListener);
		});
	}
}

export const backendClient = new BackendClient();

/*
 * WebSocket
 */

export const commands = {
	scan: () => JSON.stringify({ cmd: 'scan', data: {} }),
	connect: (address: string) => JSON.stringify({ cmd: 'connect', data: { address } })
};

export const parseMessage = (ev: MessageEvent<unknown>): ZWsMessage => {
	if (typeof ev.data !== 'string') {
		throw new Error('Invalid message format');
	}

	const json = JSON.parse(ev.data);
	return WsMessage.parse(json);
};
