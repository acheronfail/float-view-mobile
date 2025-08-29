<script lang="ts">
	import { backendClient, commands, parseMessage } from '../lib/backend-client';
	import type { ZConnectMessage, ZScanDevice, ZScanMessage } from '../lib/backend-client.types';

	let ws = $state<WebSocket | null>(null);

	let isScanning = $state(false);
	let bleDevices = $state<Array<ZScanDevice>>([]);

	function handleScanMessage(msg: ZScanMessage) {
		switch (msg.event) {
			case 'device':
				bleDevices.push(msg.data);
				break;
			case 'status':
				isScanning = msg.data.status === 'start';
				break;
			case 'error':
				alert(`Scan error: ${msg.data.message}`);
				isScanning = false;
				break;
		}
	}

	function handleConnectMessage(msg: ZConnectMessage) {}

	async function connectWebsocket() {
		if (ws) return ws;

		const result = await backendClient.connectWebsocket();
		ws = result[0];

		ws.addEventListener('message', (event) => {
			const msg = parseMessage(event);
			console.debug(msg);

			switch (msg.topic) {
				case 'socket':
					return;
				case 'scan':
					return handleScanMessage(msg);
				case 'connect':
					return handleConnectMessage(msg);
				default:
					console.warn('Unknown message topic:', msg);
					return;
			}
		});

		ws.addEventListener('close', (event) => {
			console.debug(event);
			ws = null;
		});

		return ws;
	}

	async function scanDevices() {
		try {
			const ws = await connectWebsocket();
			ws.send(commands.scan());
		} catch (err) {
			alert(String(err));
		}
	}

	async function connectBleDevice(address: string) {
		try {
			const ws = await connectWebsocket();
			ws.send(commands.connect(address));
		} catch (err) {
			alert(String(err));
		}
	}
</script>

<div class="flex flex-col items-center justify-center gap-2 p-2">
	<h1 class="text-3xl font-bold">Welcome!</h1>

	{#if ws}
		<button class="rounded border p-1" onclick={scanDevices}>scan devices</button>

		{#if isScanning}
			<div class="animate-pulse rounded border bg-yellow-200 p-1">Scanning...</div>
		{/if}

		{#if bleDevices.length > 0 || isScanning}
			<h2 class="text-2xl font-bold">Devices found:</h2>
			<ul>
				{#each bleDevices as device}
					<li>
						<button
							class="flex flex-col items-center justify-center gap-1 rounded border p-1"
							onclick={() => connectBleDevice(device.address)}
						>
							<span class="font-bold">{device.name}</span>
							<span class="text-sm">{device.address}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</div>
