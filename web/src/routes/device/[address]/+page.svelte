<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { ArrowDown, ArrowUp } from '@lucide/svelte';
	import {
		parseMessage,
		sendCommand,
		type ZCommandFirmwareResponse,
		type ZDataLogResponse
	} from '../../../lib/backend-client.types';
	import { getAppContext } from '../../../lib/context/app.svelte';
	import { goto } from '$app/navigation';
	import type { PageProps } from './$types';

	let app = getAppContext();

	let { params }: PageProps = $props();

	$effect(() => {
		console.log(app.connectedAddresses);
		if (!app.connectedAddresses.has(params.address)) {
			goto('/');
		}
	});

	let showRawData = $state(false);
	let scrolledMoreThanHalf = $state(false);

	let fw = $state<ZCommandFirmwareResponse | null>(null);
	let data = $state<ZDataLogResponse | null>(null);

	// ask backend to start logging
	$effect(() => {
		if (!app.ws) return;
		sendCommand(app.ws, 'data', { log: true, intervalMs: import.meta.env.DEV ? 1000 : 200 });
		return () => {
			if (app.ws) {
				sendCommand(app.ws, 'data', { log: false });
			}
		};
	});

	// listen for responses and data events
	$effect(() => {
		const handler = (event: MessageEvent) =>
			untrack(() => {
				const msg = parseMessage(event);
				if (msg.topic === 'command' && msg.event === 'result' && msg.data.command === 0) {
					fw = msg.data.data;
				}

				if (msg.topic === 'data') {
					switch (msg.event) {
						case 'log':
							data = msg.data;
							break;
					}
				}
			});

		app.ws?.addEventListener('message', handler);
		return () => app.ws?.removeEventListener('message', handler);
	});

	// track scroll position
	let scrollToTopTimeout = -1;
	$effect(() => {
		const updateScrollPosition = () => {
			const maxScroll = document.body.scrollHeight - window.innerHeight;
			scrolledMoreThanHalf = window.scrollY > maxScroll / 2;

			clearTimeout(scrollToTopTimeout);
			scrollToTopTimeout = window.setTimeout(() => {
				if (!showRawData) {
					window.scrollTo({ top: 0, behavior: 'smooth' });
				}
			}, 5_000);
		};

		updateScrollPosition();
		window.addEventListener('scroll', updateScrollPosition);
		return () => window.removeEventListener('scroll', updateScrollPosition);
	});

	async function runHandlingErrors(fn: () => void) {
		try {
			fn();
		} catch (err) {
			alert(String(err));
		}
	}

	const disconnect = () => {
		runHandlingErrors(
			() =>
				app.ws &&
				sendCommand(app.ws, 'disconnect', {
					address: params.address
				})
		);
	};

	const fmt = (n: number | undefined, digits = 2) => n?.toFixed(digits);

	onMount(() => {
		if (app.ws) sendCommand(app.ws, 'command', { command: 0 });
	});
</script>

<div class="flex min-h-screen w-full flex-col">
	{#if showRawData}
		<div class="p-4 font-mono text-sm">
			{#if data}
				<h3 class="mb-4 text-lg font-bold">Raw Data:</h3>
				<ul class="space-y-1">
					{#each Object.entries(data) as [key, value]}
						<li class="flex justify-between border-b border-gray-700 pb-1">
							<span class="text-primary-400">{key}:</span>
							<span class="text-success-300 text-right">{JSON.stringify(value)}</span>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="text-gray-400">No data available</p>
			{/if}
		</div>
	{:else}
		<!-- Top row: adc1, state, adc2 -->
		<div class="grid grid-cols-6 gap-2 rounded-lg p-2">
			<div class="flex items-center justify-center rounded bg-yellow-950 p-2">
				{fmt(data?.adc1) ?? '?'}
			</div>
			<div class="col-span-4 flex items-center justify-center rounded bg-green-900 p-2">
				{data?.state ?? '-'}
			</div>
			<div class="flex items-center justify-center rounded bg-yellow-950 p-2">
				{fmt(data?.adc2) ?? '?'}
			</div>
		</div>

		<!-- Middle row: three columns a, b, c -->
		<div class="grid flex-1 grid-cols-8 rounded-lg">
			<div class="flex items-center justify-center rounded">
				{fmt(data?.voltage) ?? '?'}V
			</div>
			<div class="col-span-6 flex items-center justify-center rounded">
				<!-- TODO: easy to grok gauges + pitch/roll visualisations -->
				<ul class="font-mono">
					<li>Speed: {fmt(data?.speed ?? 0)} km/h</li>
					<li>Duty: {fmt(data?.duty ?? 0, 0)}%</li>
					<li>Pitch: {fmt(data?.truePitch ?? 0)}°</li>
					<li>Roll: {fmt(data?.roll ?? 0)}°</li>
				</ul>
			</div>
			<div class="flex flex-col items-center justify-center rounded">
				<div>{fmt(data?.currentMotor) ?? '?'}A</div>
				<div>{fmt(data?.currentBattery) ?? '?'}A</div>
			</div>
		</div>

		<!-- Bottom row: connected address -->
		<div class="grid grid-cols-6 rounded-lg p-2">
			<div class="text-center">1</div>
			<div class="col-span-4 rounded p-2 text-center font-mono text-sm">
				{params.address}
			</div>
			<div></div>
		</div>
	{/if}
</div>

{#if !showRawData}
	<button
		class="btn-icon border-surface-600 bg-surface-950 fixed bottom-6 right-6 flex h-8 w-8 items-center justify-center rounded border"
		onpointerdown={() => {
			if (scrolledMoreThanHalf) {
				window.scrollTo({ top: 0, behavior: 'smooth' });
			} else {
				window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });
			}
		}}
	>
		{#if scrolledMoreThanHalf}
			<ArrowUp />
		{:else}
			<ArrowDown />
		{/if}
	</button>
{/if}

<!-- Disconnect button (scrollable area below) -->
<div
	class="border-surface-600 bg-surface-950 flex flex-col items-center justify-center gap-2 border-t py-12 text-center"
>
	{#if fw}
		<div class="text-surface-400 flex flex-col gap-1 text-sm">
			<span>Firmware: v{fw.major}.{fw.minor.toString().padStart(2, '0')}</span>
			<span>(hw: {fw.hwType}, {fw.betaVersion ? `beta${fw.betaVersion}` : ''}, id: {fw.uuid})</span>
		</div>
	{:else}
		<div class="text-surface-400 text-sm">Firmware: unknown</div>
	{/if}
	<button class="btn preset-filled-error-100-900" onpointerdown={disconnect}> disconnect </button>
	<button
		class="btn preset-filled-secondary-100-900"
		onpointerdown={() => {
			showRawData = !showRawData;
			setTimeout(() => window.scrollTo({ top: 0, behavior: 'smooth' }), 10);
		}}
	>
		{showRawData ? 'Hide Raw Data' : 'Show Raw Data'}
	</button>
</div>
