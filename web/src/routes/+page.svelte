<script lang="ts">
	import { onMount } from 'svelte';

	import { backendClient } from '../lib/backend-client';
	import { parseMessage } from '../lib/backend-client.types';
	import Home from '../lib/components/Home.svelte';
	import Device from '../lib/components/Device.svelte';

	let ws = $state<WebSocket | null>(null);

	let connectedAddress = $state<string | null>(null);

	let retries = 0;
	async function connectWebsocket() {
		if (ws) return ws;

		try {
			const result = await backendClient.connectWebsocket();
			ws = result[0];
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

		ws.addEventListener('message', (event) => {
			const msg = parseMessage(event);
			console.debug(['<', msg.topic, msg.event].join(':'), msg.data);
		});

		ws.addEventListener('close', (event) => {
			ws = null;
			connectWebsocket();
		});

		return ws;
	}

	onMount(() => {
		connectWebsocket();
	});

	// TODO: UI to be able to look at logs when boards aren't connected
	// TODO: instead of disabling scan button, change it to "cancel scan"
	// TODO: upon connection navigate to /device page
</script>

{#if connectedAddress}
	<Device {ws} bind:connectedAddress />
{:else}
	<Home {ws} bind:connectedAddress />
{/if}
