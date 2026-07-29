<script lang="ts">
	import { goto } from '$app/navigation';
	import { backendState } from '$lib/backend/state.svelte';

	$effect(() => {
		if (backendState.status === 'connected' && !backendState.snapshot?.activeConnection) {
			void goto('#/', { replaceState: true });
		}
	});

	async function disconnect() {
		await backendState.disconnectDevice();
		if (!backendState.snapshot?.activeConnection) await goto('#/', { replaceState: true });
	}
</script>

<section aria-labelledby="live-heading" class="rounded-2xl border border-outline bg-surface p-5">
	<h2 id="live-heading" class="text-xl font-medium">Live</h2>
	{#if backendState.snapshot?.activeConnection}
		<p class="mt-2 text-lg">
			{backendState.snapshot.activeConnection.name || 'VESC board'}
		</p>
		{#if backendState.snapshot.activeConnection.state === 'ready'}
			<p class="mt-1 text-success">Connected · durable session active</p>
			{#if backendState.firmware}
				<p class="mt-1 text-sm text-muted">
					{backendState.firmware.hardwareName || 'VESC'} · VESC {backendState.firmware.vescVersion}
					· {backendState.firmware.packageName}
					{backendState.firmware.packageVersion}
				</p>
			{/if}
		{:else}
			<p class="mt-1 text-warning capitalize">
				Connection state: {backendState.snapshot.activeConnection.state.replaceAll('_', ' ')}
			</p>
		{/if}
		<button
			class="mt-5 rounded-lg bg-danger px-4 py-2 text-white disabled:opacity-50"
			disabled={backendState.busy}
			onclick={disconnect}>Stop and disconnect</button
		>
	{:else}
		<p class="mt-3 text-muted">Recovering active connection…</p>
	{/if}
	{#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>
