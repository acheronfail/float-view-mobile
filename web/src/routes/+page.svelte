<script lang="ts">
	import { onMount } from 'svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { ChevronRight } from '@lucide/svelte';

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
	let bleDevices = $state<Array<ZScanDevice>>([
		{
			name: 'name',
			address: 'address:as:fd'
		}
	]);

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

<div class="flex h-screen flex-col items-center gap-2 p-2">
	<h1 class="text-3xl font-bold">Welcome!</h1>

	{#if ws}
		<div class="flex flex-row items-center gap-2">
			<button class="btn preset-filled-primary-500" onclick={scanDevices} disabled={isScanning}>
				scan devices
			</button>
			{#if isScanning}
				<ProgressRing
					value={null}
					size="size-6"
					meterStroke="stroke-primary-600-400"
					trackStroke="stroke-primary-50-950"
				/>
			{/if}
		</div>

		{#if bleDevices.length > 0 || isScanning}
			<h2 class="text-2xl font-bold">Devices found:</h2>
			<ul class="flex w-full flex-col items-center gap-2">
				{#each bleDevices as device}
					<li>
						<button
							class="btn preset-filled-surface-100-900 border-surface-600 w-lg flex flex-row items-center justify-between gap-1 border text-left shadow-xl"
							disabled={isConnecting || isConnected}
							onclick={() => connectBleDevice(device.address)}
						>
							<div class="flex w-full flex-col items-center gap-1">
								<div class="w-full font-bold">{device.name}</div>
								<div class="w-full text-sm">{device.address}</div>
							</div>
							<div>
								{#if isConnecting}
									<ProgressRing
										value={null}
										size="size-6"
										meterStroke="stroke-surface-600-400"
										trackStroke="stroke-surface-50-950"
									/>
								{:else}
									<ChevronRight class="h-6 w-6" />
								{/if}
							</div>
						</button>
					</li>
				{/each}
			</ul>
		{/if}

		{#if isConnected}
			<div class="flex flex-col items-center gap-2">
				<h2 class="text-2xl font-bold">Connected!</h2>
				<button class="btn preset-filled-error-500" onclick={disconnectBleDevice}>
					disconnect
				</button>
			</div>
		{/if}
	{/if}
</div>
