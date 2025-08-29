<script lang="ts">
	import { untrack } from 'svelte';
	import { parseMessage, sendCommand } from '../backend-client.types';

	let {
		ws,
		connectedAddress = $bindable(null)
	}: {
		ws: WebSocket | null;
		connectedAddress: string | null;
	} = $props();

	// TODO: name, realtime data, etc

	$effect(() => {
		if (!ws) return;

		const handler = (event: MessageEvent) =>
			untrack(() => {
				const msg = parseMessage(event);
				if (
					msg.topic === 'connect' &&
					msg.event === 'status' &&
					msg.data.status === 'disconnected'
				) {
					connectedAddress = null;
				}
			});

		ws.addEventListener('message', handler);
		return () => ws.removeEventListener('message', handler);
	});

	async function runHandlingErrors(fn: () => void) {
		try {
			fn();
		} catch (err) {
			alert(String(err));
		}
	}

	const disconnect = () => {
		runHandlingErrors(() => ws && sendCommand(ws, 'disconnect', {}));
	};
</script>

<div>Device</div>
<div>Connected to {connectedAddress}</div>
<div>{ws ? 'WebSocket connected' : 'WebSocket not connected'}</div>

<button class="btn preset-filled-secondary-500" onclick={disconnect}> disconnect </button>
