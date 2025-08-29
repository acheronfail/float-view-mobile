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

	let bleDevices = $state<Array<{ name: string; id: string }>>([]);
	async function scanDevices() {
		try {
			const response = await backendClient.scanBleDevices();
			if (response.kind === 'error') {
				alert(String(response.data.message));
				return;
			}
			console.log(response);
			bleDevices = response.data;
		} catch (err) {
			alert(String(err));
		}
	}

	async function connectBleDevice(id: string) {
		try {
			const response = await backendClient.connectBleDevice(id);
			if (response.kind === 'error') {
				alert(String(response.data.message));
				return;
			}

			preText = response.data.status + ' --- now use websocket?';
		} catch (err) {
			alert(String(err));
		}
	}
</script>

<div class="flex flex-col items-center justify-center gap-2 p-2">
	<h1 class="text-3xl font-bold">Welcome!</h1>

	<button class="rounded border p-1" onclick={scanDevices}>scan devices</button>
	<pre>{preText}</pre>

	{#if bleDevices.length > 0}
		<h2 class="text-2xl font-bold">BLE Devices</h2>
		<ul>
			{#each bleDevices as device}
				<li>
					<button class="rounded border p-1" onclick={() => connectBleDevice(device.id)}>
						{device.name} ({device.id})
					</button>
				</li>
			{/each}
		</ul>
	{/if}

	<button class="rounded border p-1" onclick={connectWebsocket}> connect websocket </button>
	<pre>WS: {wsState}</pre>
	<pre>{wsText}</pre>
</div>
