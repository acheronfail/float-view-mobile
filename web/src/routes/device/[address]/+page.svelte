<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { ArrowDown, ArrowUp } from '@lucide/svelte';
	import {
		parseMessage,
		type ZCommandFirmware,
		type ZRefloatRtData
	} from '../../../lib/backend-client.types';
	import { getAppContext } from '../../../lib/context/app.svelte';
	import { goto } from '$app/navigation';
	import type { PageProps } from './$types';
	import { CommPacketId } from '../../../lib/vesc.commands';
	import { RefloatCommPacketId, RefloatPackageId } from '../../../lib/refloat.commands';

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

	let fw = $state<ZCommandFirmware | null>(null);
	let rtData = $state<ZRefloatRtData | null>(null);

	let ready = $derived(() => !!fw);

	// ask backend to start logging
	$effect(() => {
		// only start logging when everything is setup
		if (!app.ws || !ready) return;

		app.sendCommand({
			kind: 'logStart',
			address: params.address,
			intervalMs: import.meta.env.DEV ? 1000 : 200
		});

		return () => {
			app.sendCommand({ kind: 'logStop', address: params.address });
		};
	});

	// listen for responses and data events
	$effect(() => {
		const handler = (event: MessageEvent) =>
			untrack(() => {
				const msg = parseMessage(event);
				if (msg.kind === 'commandError') {
					alert(`Command error: ${msg.message}`);
				} else if (msg.kind === 'commandFirmware' && msg.address === params.address) {
					fw = msg.firmware;
				} else if (msg.kind === 'logData' && msg.address === params.address) {
					rtData = msg.data;
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
		runHandlingErrors(() => app.sendCommand({ kind: 'disconnect', address: params.address }));
	};

	const fmt = (n: number | undefined, digits = 2) => n?.toFixed(digits);

	onMount(() => {
		// TODO: open modal while all these initial things load here
		// TODO: get float/refloat config and versions
		// TODO: get app config
		// TODO: get motor config
		// TODO: basic checks?

		// TODO: need to send synchronously!

		app.sendCommand({
			kind: 'command',
			address: params.address,
			cmd: { kind: CommPacketId.COMM_FW_VERSION }
		});

		setTimeout(() => {
			app.sendCommand({
				kind: 'command',
				address: params.address,
				cmd: {
					kind: CommPacketId.COMM_CUSTOM_APP_DATA,
					appId: RefloatPackageId,
					appCommand: { kind: RefloatCommPacketId.COMMAND_GET_RTDATA }
				}
			});
		}, 1000);
	});
</script>

<div class="flex min-h-screen w-full flex-col">
	{#if showRawData}
		<div class="p-4 font-mono text-sm">
			{#if rtData}
				<h3 class="mb-4 text-lg font-bold">Raw Data:</h3>
				<ul class="space-y-1">
					{#each Object.entries(rtData) as [key, value]}
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
				{fmt(rtData?.adc1) ?? '?'}
			</div>
			<div class="col-span-4 flex items-center justify-center rounded bg-green-900 p-2">
				{rtData?.state ?? '-'}
			</div>
			<div class="flex items-center justify-center rounded bg-yellow-950 p-2">
				{fmt(rtData?.adc2) ?? '?'}
			</div>
		</div>

		<!-- Middle row: three columns a, b, c -->
		<div class="grid flex-1 grid-cols-8 rounded-lg">
			<div class="flex items-center justify-center rounded">
				{fmt(rtData?.voltage) ?? '?'}V
			</div>
			<div class="col-span-6 flex items-center justify-center rounded">
				<!-- TODO: easy to grok gauges + pitch/roll visualisations -->
				<ul class="font-mono">
					<li>Speed: {fmt(rtData?.speed ?? 0)} km/h</li>
					<li>Duty: {fmt(rtData?.duty ?? 0, 0)}%</li>
					<li>Pitch: {fmt(rtData?.truePitch ?? 0)}°</li>
					<li>Roll: {fmt(rtData?.roll ?? 0)}°</li>
				</ul>
			</div>
			<div class="flex flex-col items-center justify-center rounded">
				<div>{fmt(rtData?.currentMotor) ?? '?'}A</div>
				<div>{fmt(rtData?.currentBattery) ?? '?'}A</div>
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
