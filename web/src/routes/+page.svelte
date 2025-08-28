<script lang="ts">
	import { backendClient } from '../lib/backend-client';

	let preText = $state('');
	let wsText = $state('');
	let wsState = $state('disconnected');

	let ws = $state<WebSocket | null>(null);

	async function connectWebsocket() {
		if (!ws) {
			wsState = 'connecting';
			const result = await backendClient.connectWebsocket();
			ws = result[0];
			ws.addEventListener('message', (event) => {
				const data = JSON.parse(event.data);
				wsText += JSON.stringify(data, null, 2);
			});
			ws.addEventListener('close', (event) => {
				wsState = 'disconnected';
				ws = null;
				wsText += '--- CLOSED: ' + event.code + ' ' + event.reason + ' ---\n';
			});
			wsState = 'connected';
		} else {
			ws.close(1000, 'User disconnected');
			ws = null;
			wsState = 'disconnected';
			wsText = '';
		}
	}

	async function pingStatus() {
		try {
			const status = await backendClient.status();
			preText = JSON.stringify(status, null, 2);
		} catch (err) {
			alert(String(err));
		}
	}
</script>

<div class="flex flex-col items-center justify-center gap-2 p-2">
	<h1 class="text-3xl font-bold">Welcome!</h1>

	<button class="rounded border p-1" onclick={pingStatus}> ping status </button>
	<pre>{preText}</pre>

	<button class="rounded border p-1" onclick={connectWebsocket}> connect websocket </button>
	<pre>WS: {wsState}</pre>
	<pre>{wsText}</pre>
</div>
