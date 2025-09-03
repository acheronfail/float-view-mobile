<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { initAppContext } from '../lib/context/app.svelte';
	import { backendClient } from '../lib/backend-client';
	import { parseMessage } from '../lib/backend-client.types';

	let { children } = $props();

	let app = initAppContext();

	let retries = 0;
	async function connectWebsocket() {
		if (app.ws) return;

		try {
			const [ws] = await backendClient.connectWebsocket();
			app.ws = ws;
		} catch (error) {
			console.error('Failed to connect to websocket', error);
			if (retries < 5) {
				retries++;
				setTimeout(connectWebsocket, 500 * retries * retries);
			}

			console.error('Max retries reached!');
			return;
		}

		retries = 0;

		app.ws.addEventListener('message', (event) => {
			const msg = parseMessage(event);
			console.debug(['<', msg.topic, msg.event].join(':'), msg.data);

			if (msg.topic === 'disconnect' && msg.event === 'complete') {
				app.connectedAddresses.delete(msg.data.address);

				// TODO: be able to differentiate between user-initiated disconnect and error
				if (msg.data.reason !== 'websocket requested') {
					setTimeout(() => alert(`Disconnected: ${msg.data.reason ?? 'unknown reason'}`));
				}
			}
		});

		app.ws.addEventListener('close', (event) => {
			app.ws = null;
			connectWebsocket();
		});
	}

	connectWebsocket();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<div
	class="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 dark:from-gray-900 dark:to-gray-800"
>
	{@render children?.()}
</div>
