<script lang="ts">
	import { onMount } from 'svelte';
	import { apiStore } from '$lib/api/store.svelte';

	let selectedTab = $state<'device' | 'sessions'>('device');

	onMount(() => {
		void apiStore.connect();
		return () => apiStore.close();
	});
</script>

<div class="mx-auto flex min-h-screen max-w-3xl flex-col px-4 py-6">
	<header class="mb-6">
		<h1 class="text-3xl font-semibold">Float View</h1>
		<p class="text-sm text-slate-400">Durable VESC sessions</p>
	</header>

	<nav aria-label="Primary" class="mb-6 grid grid-cols-2 rounded-xl bg-slate-900 p-1">
		<button
			class:!bg-indigo-600={selectedTab === 'device'}
			class="rounded-lg px-4 py-2"
			onclick={() => (selectedTab = 'device')}>Device</button
		>
		<button
			class:!bg-indigo-600={selectedTab === 'sessions'}
			class="rounded-lg px-4 py-2"
			onclick={() => (selectedTab = 'sessions')}>Sessions</button
		>
	</nav>

	{#if selectedTab === 'device'}
		<section aria-labelledby="device-heading" class="rounded-2xl bg-slate-900 p-5">
			{#if apiStore.status === 'connected'}
				{#if apiStore.snapshot?.activeConnection}
					<h2 id="device-heading" class="text-xl font-medium">Live</h2>
					<p class="mt-2 text-lg">{apiStore.snapshot.activeConnection.name || 'VESC board'}</p>
					{#if apiStore.snapshot.activeConnection.state === 'ready'}
						<p class="mt-1 text-emerald-300">Connected · durable session active</p>
						{#if apiStore.firmware}
							<p class="mt-1 text-sm text-slate-300">
								{apiStore.firmware.hardwareName || 'VESC'} · VESC {apiStore.firmware.vescVersion}
								· {apiStore.firmware.packageName}
								{apiStore.firmware.packageVersion}
							</p>
						{/if}
					{:else}
						<p class="mt-1 text-amber-300">
							Connection state: {apiStore.snapshot.activeConnection.state.replaceAll('_', ' ')}
						</p>
					{/if}
					<button
						class="mt-5 rounded-lg bg-rose-700 px-4 py-2 disabled:opacity-50"
						disabled={apiStore.busy}
						onclick={() => apiStore.disconnectDevice()}>Stop and disconnect</button
					>
				{:else}
					<h2 id="device-heading" class="text-xl font-medium">Saved devices</h2>
					{#if apiStore.savedDevices.length}
						<ul class="mt-3 space-y-2">
							{#each apiStore.savedDevices as device (device.savedDeviceId)}
								<li class="flex items-center justify-between rounded-lg bg-slate-800 p-3">
									<span>{device.displayName}</span>
									<span class="text-sm text-slate-400">
										{device.profileComplete ? 'Configured' : 'Setup available'}
									</span>
								</li>
							{/each}
						</ul>
					{:else}
						<p class="mt-3 text-slate-300">No saved boards yet.</p>
					{/if}
					<button
						class="mt-4 rounded-lg bg-indigo-600 px-4 py-2 disabled:opacity-50"
						disabled={apiStore.busy}
						onclick={() => apiStore.scan()}>Scan for a board</button
					>
					{#if apiStore.discoveredDevices.length}
						<h3 class="mt-6 font-medium">Nearby boards</h3>
						<ul class="mt-2 space-y-2">
							{#each apiStore.discoveredDevices as device (device.deviceId)}
								<li class="flex items-center justify-between rounded-lg bg-slate-800 p-3">
									<span>{device.name}</span>
									<button
										class="rounded-md bg-indigo-600 px-3 py-1 disabled:opacity-50"
										disabled={apiStore.busy}
										onclick={() => apiStore.connectDevice(device.deviceId)}>Connect</button
									>
								</li>
							{/each}
						</ul>
					{/if}
				{/if}
				{#if apiStore.error}<p role="alert" class="mt-3 text-amber-300">{apiStore.error}</p>{/if}
			{:else if apiStore.status === 'failed'}
				<p role="alert" class="mt-3 text-amber-300">{apiStore.error}</p>
				<button class="mt-4 rounded-lg bg-slate-700 px-4 py-2" onclick={() => apiStore.connect()}
					>Retry</button
				>
			{:else}
				<p class="mt-3 text-slate-400">Starting native backend…</p>
			{/if}
		</section>
	{:else}
		<section aria-labelledby="sessions-heading" class="rounded-2xl bg-slate-900 p-5">
			{#if apiStore.selectedSession}
				<button class="mb-4 text-indigo-300" onclick={() => apiStore.closeSession()}
					>← Sessions</button
				>
				<h2 id="sessions-heading" class="text-xl font-medium">Review</h2>
				<p class="mt-2 text-lg">{apiStore.selectedSession.displayName}</p>
				<div class="mt-4 grid grid-cols-2 gap-3">
					<div class="rounded-lg bg-slate-800 p-3">
						<p class="text-sm text-slate-400">Samples</p>
						<p class="text-xl">{apiStore.selectedSession.sampleCount}</p>
					</div>
					<div class="rounded-lg bg-slate-800 p-3">
						<p class="text-sm text-slate-400">Distance</p>
						<p class="text-xl">
							{((apiStore.selectedSession.distanceM ?? 0) / 1000).toFixed(2)} km
						</p>
					</div>
				</div>
				<div class="mt-5 rounded-lg border border-slate-700 p-4">
					<h3 class="font-medium">Ride trends</h3>
					<p class="mt-2 text-sm text-slate-400">
						Bounded series queries power the Svelte/SVG charts here.
					</p>
					<button class="mt-3 rounded-md bg-slate-700 px-3 py-2">Open diagnostic zoom</button>
				</div>
			{:else}
				<h2 id="sessions-heading" class="text-xl font-medium">Sessions</h2>
				<div class="mt-3 flex gap-2 text-sm">
					<button class="rounded-md bg-slate-800 px-3 py-2">Device</button>
					<button class="rounded-md bg-slate-800 px-3 py-2">Date</button>
					<button class="rounded-md bg-slate-800 px-3 py-2">Duration</button>
					<button class="rounded-md bg-slate-800 px-3 py-2">Distance</button>
				</div>
				{#if apiStore.sessions.length}
					<ul class="mt-4 space-y-2">
						{#each apiStore.sessions as session (session.sessionId)}
							<li>
								<button
									class="w-full rounded-lg bg-slate-800 p-3 text-left"
									onclick={() => apiStore.openSession(session.sessionId)}
								>
									<span class="block font-medium">{session.displayName}</span>
									<span class="text-sm text-slate-400">
										{session.state} · {Math.round(session.totalElapsedMs / 60000)} min ·
										{((session.distanceM ?? 0) / 1000).toFixed(2)} km
									</span>
								</button>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="mt-3 text-slate-300">Recorded sessions will appear here.</p>
				{/if}
			{/if}
		</section>
	{/if}
</div>
