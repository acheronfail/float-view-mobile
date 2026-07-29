<script lang="ts">
	import { goto } from '$app/navigation';
	import { backendState } from '$lib/backend/state.svelte';

	async function connect(deviceId: string) {
		if (await backendState.connectDevice(deviceId)) await goto('#/live');
	}
</script>

<section aria-labelledby="home-heading" class="rounded-2xl border border-outline bg-surface p-5">
	<h2 id="home-heading" class="text-xl font-medium">Connect a board</h2>

	{#if backendState.status === 'connected'}
		<p class="mt-2 text-muted">
			Scan for a nearby VESC board. Bluetooth and durable logging remain owned by the native app.
		</p>
		<button
			class="mt-5 rounded-lg bg-accent px-4 py-2 text-white disabled:opacity-50"
			disabled={backendState.busy || backendState.scanning}
			onclick={() => backendState.scan()}
			>{backendState.scanning ? 'Scanning…' : 'Scan for a board'}</button
		>

		{#if backendState.discoveredDevices.length}
			<h3 class="mt-6 font-medium">Nearby boards</h3>
			<ul class="mt-2 space-y-2">
				{#each backendState.discoveredDevices as device (device.deviceId)}
					<li class="flex items-center justify-between gap-3 rounded-lg bg-elevated p-3">
						<div>
							<span class="block">{device.name}</span>
							<span class="text-sm text-muted capitalize">{device.state.replaceAll('_', ' ')}</span>
						</div>
						<button
							class="rounded-md bg-accent px-3 py-1 text-white disabled:opacity-50"
							disabled={backendState.busy}
							onclick={() => connect(device.deviceId)}>Connect</button
						>
					</li>
				{/each}
			</ul>
		{/if}
	{:else if backendState.status === 'failed'}
		<p role="alert" class="mt-3 text-warning">{backendState.error}</p>
		<button class="mt-4 rounded-lg bg-elevated px-4 py-2" onclick={() => backendState.connect()}
			>Retry</button
		>
	{:else}
		<p class="mt-3 text-muted">Starting native backend…</p>
	{/if}

	{#if backendState.error && backendState.status !== 'failed'}
		<p role="alert" class="mt-3 text-warning">{backendState.error}</p>
	{/if}
</section>
