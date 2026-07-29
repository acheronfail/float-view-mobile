<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { backendState } from '$lib/backend/state.svelte';
	import {
		normalizedCustomName,
		SerialSaveQueue,
		validParallelCount,
		validSeriesCount
	} from './autosave';

	let {
		savedDeviceId,
		embedded = false,
		onclose
	}: { savedDeviceId: string; embedded?: boolean; onclose?: () => void } = $props();
	let customDisplayName = $state('');
	let seriesCount = $state<number | undefined>();
	let parallelCount = $state<number | undefined>();
	let motor = $state('');
	const saveQueue = new SerialSaveQueue();

	const device = $derived(
		backendState.savedDevices.find((value) => value.savedDeviceId === savedDeviceId)
	);

	onMount(() => {
		let active = true;
		void backendState.openBoardProfile(savedDeviceId).then((opened) => {
			if (!active) return;
			if (!opened) {
				if (embedded) onclose?.();
				else void goto('#/boards', { replaceState: true });
				return;
			}
			customDisplayName = device?.customDisplayName ?? '';
			const values = backendState.selectedBoardProfile?.values;
			seriesCount = typeof values?.seriesCount === 'number' ? values.seriesCount : undefined;
			parallelCount = typeof values?.parallelCount === 'number' ? values.parallelCount : undefined;
			motor = typeof values?.motor === 'string' ? values.motor : '';
		});
		return () => {
			active = false;
			backendState.closeBoardProfile();
		};
	});

	function enqueue(action: () => Promise<void>) {
		saveQueue.enqueue(action);
	}

	function saveName() {
		const next = normalizedCustomName(customDisplayName);
		if (next === (device?.customDisplayName ?? null)) return;
		enqueue(() => backendState.updateSavedDeviceName(savedDeviceId, next));
	}

	function saveProfile(patch: Record<string, unknown>) {
		enqueue(() => backendState.updateBoardProfile(patch));
	}

	export async function flush() {
		await saveQueue.flush();
	}
</script>

<section
	aria-labelledby="board-heading"
	class:rounded-2xl={!embedded}
	class:border={!embedded}
	class:border-outline={!embedded}
	class:bg-surface={!embedded}
	class:p-5={!embedded}
>
	{#if !embedded}<a class="text-accent" href="#/boards">← Boards</a>{/if}
	{#if backendState.selectedBoardProfile && device}
		<h2 id="board-heading" class:mt-3={!embedded} class="text-xl font-medium">Configure board</h2>
		<p class="mt-1 text-sm text-muted">Changes save automatically on this device.</p>
		<div class="mt-4 grid gap-3 sm:grid-cols-2">
			<label class="text-sm sm:col-span-2">
				<span class="mb-1 block text-muted">Display name</span>
				<input
					class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
					type="text"
					maxlength="128"
					bind:value={customDisplayName}
					placeholder={device.advertisedName ?? device.bluetoothIdentifier}
					onblur={saveName}
				/>
			</label>
			<div class="rounded-md border border-outline bg-elevated px-3 py-2 text-sm sm:col-span-2">
				<span class="block text-muted">Bluetooth identifier</span>
				<code class="mt-1 block break-all text-foreground">{device.bluetoothIdentifier}</code>
			</div>
			<label class="text-sm">
				<span class="mb-1 block text-muted">Battery series cells (S)</span>
				<input
					class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
					type="number"
					min="1"
					max="36"
					bind:value={seriesCount}
					onblur={() => {
						if (validSeriesCount(seriesCount)) saveProfile({ seriesCount });
					}}
				/>
			</label>
			<label class="text-sm">
				<span class="mb-1 block text-muted">Battery parallel cells (P)</span>
				<input
					class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
					type="number"
					min="1"
					max="20"
					bind:value={parallelCount}
					onblur={() => {
						if (validParallelCount(parallelCount)) saveProfile({ parallelCount });
					}}
				/>
			</label>
			<label class="text-sm sm:col-span-2">
				<span class="mb-1 block text-muted">Motor</span>
				<select
					class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
					bind:value={motor}
					onchange={() => {
						if (motor) saveProfile({ motor });
					}}
				>
					<option value="">Choose a motor</option>
					<option value="hypercore_xr">Hypercore (XR)</option>
					<option value="superflux_hs">Superflux HS</option>
					<option value="custom">Custom / other</option>
				</select>
			</label>
		</div>
		{#if backendState.selectedBoardProfile.missingFields.length}
			<p class="mt-3 text-sm text-warning">
				Missing: {backendState.selectedBoardProfile.missingFields.join(', ')}
			</p>
		{:else}
			<p class="mt-3 text-sm text-success">Core board setup complete</p>
		{/if}
	{:else}
		<p class="text-muted">Loading board configuration…</p>
	{/if}
	{#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>
