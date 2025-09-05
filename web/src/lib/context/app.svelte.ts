import { getContext, setContext } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';
import type { ZBackendCommand } from '../backend-client.types';

export interface AppContext {
	/**
	 * WebSocket connection to the backend app server.
	 */
	ws: WebSocket | null;
	/**
	 * A set of currently connected devices.
	 */
	connectedAddresses: Set<string>;
	/**
	 *
	 */
	sendCommand(command: ZBackendCommand): void;
}

const key = Symbol('AppContext');

export function initAppContext(): AppContext {
	const ctx = $state<AppContext>({
		ws: null,
		connectedAddresses: new SvelteSet(),
		sendCommand(command) {
			if (this.ws) {
				console.debug('>send', command);
				this.ws.send(JSON.stringify(command));
			} else {
				console.warn('>X', command);
			}
		}
	});
	return setContext(key, ctx);
}

export function getAppContext() {
	return getContext<AppContext>(key);
}
