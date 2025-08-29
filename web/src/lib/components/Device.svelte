<script lang="ts">
	import { untrack } from 'svelte';
	import { ArrowDown, ArrowUp } from '@lucide/svelte';
	import { parseMessage, sendCommand } from '../backend-client.types';

	let {
		ws,
		connectedAddress = $bindable(null)
	}: {
		ws: WebSocket | null;
		connectedAddress: string | null;
	} = $props();

	// TODO: name, realtime data, etc
	let showRawData = $state(false);
	let scrolledMoreThanHalf = $state(false);

	// listen for disconnect event
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

	// track scroll position
	$effect(() => {
		const updateScrollPosition = () => {
			const maxScroll = document.body.scrollHeight - window.innerHeight;
			scrolledMoreThanHalf = window.scrollY > maxScroll / 2;
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
		runHandlingErrors(() => ws && sendCommand(ws, 'disconnect', {}));
	};
</script>

<div class="flex min-h-screen w-full flex-col">
	{#if showRawData}
		<pre class="font-mono">TODO: dump all the data here</pre>
	{:else}
		<!-- Top row: adc1, state, adc2 -->
		<div class="grid grid-cols-6 gap-2 rounded-lg p-2">
			<div class="flex items-center justify-center rounded bg-yellow-950 p-2">adc1</div>
			<div class="col-span-4 flex items-center justify-center rounded bg-green-900 p-2">state</div>
			<div class="flex items-center justify-center rounded bg-yellow-950 p-2">adc2</div>
		</div>

		<!-- Middle row: three columns a, b, c -->
		<div class="grid flex-1 grid-cols-8 rounded-lg">
			<div class="flex items-center justify-center rounded">TODO</div>
			<div class="col-span-6 flex items-center justify-center rounded">
				TODO: speedo/pitch/duty/etc
			</div>
			<div class="flex items-center justify-center rounded">TODO</div>
		</div>

		<!-- Bottom row: connected address -->
		<div class="grid grid-cols-6 rounded-lg p-2">
			<div class="text-center">1</div>
			<div class="col-span-4 rounded p-2 text-center font-mono text-sm">
				{connectedAddress}
			</div>
			<div></div>
		</div>
	{/if}
</div>

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

<!-- Disconnect button (scrollable area below) -->
<div
	class="border-surface-600 bg-surface-950 flex flex-col items-center justify-center gap-2 border-t py-12 text-center"
>
	<button class="btn preset-filled-error-100-900" onpointerdown={disconnect}> disconnect </button>
	<button
		class="btn preset-filled-secondary-100-900"
		onpointerdown={() => (showRawData = !showRawData)}
	>
		{showRawData ? 'Hide Raw Data' : 'Show Raw Data'}
	</button>
</div>
