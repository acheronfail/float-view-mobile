<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { backendState } from '$lib/backend/state.svelte';

	let { savedDeviceId }: { savedDeviceId: string } = $props();
	let seriesCount = $state<number | undefined>();
	let parallelCount = $state<number | undefined>();
	let motor = $state('');

	onMount(() => {
		let active = true;
		void backendState.openBoardProfile(savedDeviceId).then((opened) => {
			if (!active) return;
			if (!opened) {
				void goto('#/boards', { replaceState: true });
				return;
			}
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

	async function save() {
		if (
			seriesCount === undefined ||
			parallelCount === undefined ||
			!Number.isInteger(seriesCount) ||
			!Number.isInteger(parallelCount) ||
			seriesCount < 1 ||
			seriesCount > 36 ||
			parallelCount < 1 ||
			parallelCount > 20 ||
			!motor
		)
			return;
		await backendState.updateBoardProfile({ seriesCount, parallelCount, motor });
	}
</script>

<section aria-labelledby="board-heading" class="rounded-2xl border border-outline bg-surface p-5">
	<a class="text-accent" href="#/boards">← Boards</a>
	{#if backendState.selectedBoardProfile}
		<h2 id="board-heading" class="mt-3 text-xl font-medium">Configure board</h2>
		<p class="mt-1 text-sm text-muted">
			Local app metadata · revision {backendState.selectedBoardProfile.revision}
		</p>
		<div class="mt-4 grid gap-3 sm:grid-cols-2">
			<label class="text-sm">
				<span class="mb-1 block text-muted">Battery series cells (S)</span>
				<input
					class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
					type="number"
					min="1"
					max="36"
					bind:value={seriesCount}
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
				/>
			</label>
			<label class="text-sm sm:col-span-2">
				<span class="mb-1 block text-muted">Motor</span>
				<select
					class="w-full rounded-md border border-outline bg-elevated px-3 py-2"
					bind:value={motor}
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
		<button
			class="mt-4 rounded-lg bg-accent px-4 py-2 text-white disabled:opacity-50"
			disabled={backendState.busy}
			onclick={save}>Save board settings</button
		>
	{:else}
		<p class="mt-3 text-muted">Loading Board Profile…</p>
	{/if}
	{#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>
