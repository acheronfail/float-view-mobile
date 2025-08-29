<script lang="ts">
	import { onMount } from 'svelte';
	import { backendClient } from '../lib/backend-client';
	import {
		parseMessage,
		sendCommand,
		type ZConnectMessage,
		type ZScanDevice,
		type ZScanMessage
	} from '../lib/backend-client.types';

	let ws = $state<WebSocket | null>(null);

	let isScanning = $state(false);
	let isConnecting = $state(false);
	let isConnected = $state(false);
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

	function handleConnectMessage(msg: ZConnectMessage) {
		switch (msg.event) {
			case 'status':
				isConnecting = msg.data.status === 'start' || msg.data.status === 'connecting';
				isConnected = msg.data.status === 'connected';
				break;
			case 'error':
				alert(`Connect error: ${msg.data.message}`);
				isConnected = false;
				break;
		}
	}

	async function connectWebsocket() {
		if (ws) return ws;

		const result = await backendClient.connectWebsocket();
		ws = result[0];

		ws.addEventListener('message', (event) => {
			const msg = parseMessage(event);
			console.debug(['ws', msg.topic, msg.event].join(':'), msg.data);

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
			ws = null;
			connectWebsocket();
		});

		return ws;
	}

	async function runHandlingErrors(fn: (ws: WebSocket) => void) {
		try {
			fn(await connectWebsocket());
		} catch (err) {
			alert(String(err));
		}
	}

	const scanDevices = () => {
		bleDevices = [];
		runHandlingErrors((ws) => sendCommand(ws, 'scan', {}));
	};

	const connectBleDevice = (address: string) => {
		runHandlingErrors((ws) => sendCommand(ws, 'connect', { address }));
	};

	const disconnectBleDevice = () => {
		runHandlingErrors((ws) => sendCommand(ws, 'disconnect', {}));
	};

	onMount(() => {
		connectWebsocket();
	});
</script>

<div class="flex flex-col items-center justify-center gap-2 p-2">
	<h1 class="text-3xl font-bold">Welcome!</h1>

	{#if ws}
		<button class="rounded border p-1" onclick={scanDevices}>scan devices</button>

		{#if bleDevices.length > 0 || isScanning}
			<h2 class="text-2xl font-bold">Devices found:</h2>
			<ul class="flex w-full flex-col items-center gap-2">
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

		{#if isScanning}
			<div class="animate-pulse rounded border bg-yellow-200 p-1">Scanning...</div>
		{/if}

		{#if isConnecting}
			<div class="animate-pulse rounded border bg-yellow-200 p-1">Connecting...</div>
		{/if}

		{#if isConnected}
			<button class="rounded border p-1" onclick={disconnectBleDevice}>disconnect</button>
		{/if}
	{/if}
</div>
