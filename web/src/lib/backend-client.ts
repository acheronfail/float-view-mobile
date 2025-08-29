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

	public async status() {
		return this.fetch('/api/status');
	}

	public async scanBleDevices() {
		return this.fetch('/api/ble/scan');
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