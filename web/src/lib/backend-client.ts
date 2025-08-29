import { WsMessage, type ZWsMessage } from './backend-client.types';

/**
 * Client to communicate with the iOS/Android backends
 */
class BackendClient {
	constructor() {}

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
