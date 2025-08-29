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
