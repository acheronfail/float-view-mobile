<script lang="ts">
	import { SearchX, ChevronRight } from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { flip } from 'svelte/animate';
	import { slide, fade } from 'svelte/transition';
	import {
		parseMessage,
		sendCommand,
		type ZConnectMessage,
		type ZScanDevice,
		type ZScanMessage
	} from '../backend-client.types';
	import { untrack } from 'svelte';

	let {
		ws,
		connectedAddress = $bindable(null)
	}: {
		ws: WebSocket | null;
		connectedAddress: string | null;
	} = $props();

	let isScanning = $state(false);
	let isConnecting = $state(false);
	let connectingAddress = $state<string | null>(null);
	let bleDevices = $state<Array<ZScanDevice>>([]);

	$effect(() => {
		if (!ws) return;

		const handler = (event: MessageEvent) =>
			untrack(() => {
				const msg = parseMessage(event);
				switch (msg.topic) {
					case 'scan':
						return handleScanMessage(msg);
					case 'connect':
						return handleConnectMessage(msg);
				}
			});

		ws.addEventListener('message', handler);
		return () => ws.removeEventListener('message', handler);
	});

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
				if (msg.data.status === 'ready') {
					bleDevices = bleDevices.filter((d) => d.address === connectingAddress);
					isConnecting = false;
					connectedAddress = connectingAddress;
					connectingAddress = null;
				}
				break;
			case 'disconnect':
				connectedAddress = null;
				break;
			case 'error':
				alert(`Connect error: ${msg.data.message}`);
				connectingAddress = null;
				break;
		}
	}

	async function runHandlingErrors(fn: () => void) {
		try {
			fn();
		} catch (err) {
			alert(String(err));
		}
	}

	const scanDevices = () => {
		bleDevices = [];
		runHandlingErrors(() => ws && sendCommand(ws, 'scan', {}));
	};

	const connectBleDevice = (address: string) => {
		connectingAddress = address;
		runHandlingErrors(() => ws && sendCommand(ws, 'connect', { address }));
	};
</script>

<div class="container mx-auto max-w-md px-4 py-6">
	<!-- Header -->
	<div class="mb-8 text-center">
		<h1
			class="bg-gradient-to-r from-blue-600 via-purple-600 to-indigo-600 bg-clip-text text-4xl font-bold text-transparent"
		>
			Float View
		</h1>
		<p class="text-sm text-gray-600 dark:text-gray-300">A simple VESC companion</p>
	</div>

	{#if ws}
		<!-- Devices Section -->
		<div class="mb-6 rounded-2xl bg-white p-6 shadow-lg dark:bg-gray-800">
			<h2 class="mb-4 text-center text-xl font-bold text-gray-800 dark:text-white">
				Devices Found ({bleDevices.length})
			</h2>

			<div class="mb-4 flex justify-center">
				<button
					class="btn btn-sm preset-filled-primary-500 text-lg font-semibold disabled:cursor-not-allowed disabled:opacity-50"
					onpointerdown={scanDevices}
					disabled={isScanning}
				>
					{isScanning ? 'Looking...' : 'Look for VESCs'}
				</button>
			</div>

			{#if bleDevices.length === 0}
				<div class="py-8 text-center text-gray-500 dark:text-gray-400">
					<div class="mx-auto mb-3 h-16 w-16 opacity-50">
						<SearchX class="h-16 w-16" />
					</div>
					<p class="text-sm">No devices found yet</p>
					<p class="mt-1 text-xs">Tap scan to discover nearby devices</p>
				</div>
			{:else}
				<ul class="space-y-3">
					{#each bleDevices as device (device.address)}
						<li animate:flip in:slide out:fade>
							<button
								class="card preset-filled-surface-100-900 border-surface-200 dark:border-surface-700 w-full border p-2 text-left shadow-xl hover:shadow-md disabled:cursor-not-allowed disabled:opacity-50"
								disabled={isConnecting}
								onpointerdown={() => connectBleDevice(device.address)}
							>
								<div class="flex w-full items-center justify-between">
									<div class="min-w-0 flex-1">
										<div class="truncate font-semibold text-gray-800 dark:text-white">
											{device.name || 'Unknown Device'}
										</div>
										<div class="font-mono text-sm text-gray-500 dark:text-gray-400">
											{device.address}
										</div>
									</div>
									<div class="ml-4 flex-shrink-0">
										{#if connectingAddress === device.address}
											<ProgressRing
												value={null}
												size="size-6"
												meterStroke="stroke-surface-600-400"
												trackStroke="stroke-surface-50-950"
											/>
										{:else}
											<ChevronRight class="h-5 w-5 text-gray-400 dark:text-gray-500" />
										{/if}
									</div>
								</div>
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{:else}
		<!-- Loading State -->
		<div class="rounded-2xl bg-white p-8 shadow-lg dark:bg-gray-800">
			<div class="flex flex-col items-center justify-center text-center">
				<ProgressRing
					value={null}
					size="size-8"
					meterStroke="stroke-primary-600-400"
					trackStroke="stroke-primary-50-950"
				/>
				<p class="mt-4 text-gray-600 dark:text-gray-300">Starting up...</p>
			</div>
		</div>
	{/if}
</div>
