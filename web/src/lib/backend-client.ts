import { connectApiClient } from './api/client';

/**
 * Client to communicate with the iOS/Android backends
 */
class BackendClient {
	constructor() {}

	public connectWebsocket() {
		return new Promise<[WebSocket, Event]>((resolve, reject) => {
			const ws = new WebSocket(this.webSocketUrl('/ws'));

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

	public connectApiV1(timeoutMs = 5_000) {
		return connectApiClient(this.webSocketUrl('/api/v1/ws'), timeoutMs);
	}

	private webSocketUrl(path: string) {
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		return `${protocol}//${window.location.host}${path}`;
	}
}

export const backendClient = new BackendClient();
