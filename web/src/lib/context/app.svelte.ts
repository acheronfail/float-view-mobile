import { getContext, setContext } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';

export interface AppContext {
	/**
	 * WebSocket connection to the backend app server.
	 */
	ws: WebSocket | null;
	/**
	 * A set of currently connected devices.
	 */
	connectedAddresses: Set<string>;
}

const key = Symbol('AppContext');

export function initAppContext(): AppContext {
	const ctx = $state<AppContext>({ ws: null, connectedAddresses: new SvelteSet() });
	return setContext(key, ctx);
}

export function getAppContext() {
	return getContext<AppContext>(key);
}
