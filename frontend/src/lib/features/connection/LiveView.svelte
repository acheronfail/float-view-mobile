<script lang="ts">
	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import { backendState } from '$lib/backend/state.svelte';
	import BoardEditor from '$lib/features/boards/BoardEditor.svelte';
	import {
		formatDistanceM,
		formatSpeedKph,
		formatTemperatureC
	} from '$lib/features/measurements/format';
	import TelemetrySection from './TelemetrySection.svelte';
	import { readingsAreDimmed, shouldConfirmDisconnect } from './live';

	let menuOpen = $state(false);
	let configureOpen = $state(false);
	let menuButton = $state<HTMLButtonElement>();
	let sheetCloseButton = $state<HTMLButtonElement>();
	let editor = $state<BoardEditor>();
	const connection = $derived(backendState.snapshot?.activeConnection);
	const sample = $derived(connection?.latestSample);
	const session = $derived(connection?.activeSession);
	const units = $derived(backendState.settings?.unitPreference ?? 'metric');
	const live = $derived(connection?.state === 'ready');
	const dimmed = $derived(connection ? readingsAreDimmed(connection) : false);
	const savedDevice = $derived(
		backendState.savedDevices.find((device) => device.savedDeviceId === connection?.savedDeviceId)
	);

	$effect(() => {
		if (backendState.status === 'connected' && !connection) {
			void goto('#/', { replaceState: true });
		}
	});

	function number(value: number | null | undefined, digits = 1) {
		return value == null ? '—' : value.toFixed(digits);
	}

	function speed(value: number | null | undefined) {
		return formatSpeedKph(value, units);
	}

	function temperature(value: number | null | undefined) {
		return formatTemperatureC(value, units);
	}

	function distance(value: number | null | undefined) {
		return formatDistanceM(value, units);
	}

	function duration(value: number | null | undefined) {
		if (value == null) return '—';
		const seconds = Math.floor(value / 1000);
		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		return `${hours.toString().padStart(2, '0')}:${minutes
			.toString()
			.padStart(2, '0')}:${(seconds % 60).toString().padStart(2, '0')}`;
	}

	async function disconnect() {
		menuOpen = false;
		if (
			shouldConfirmDisconnect(sample?.rideState) &&
			!window.confirm('The board is not idle. Stop the active session and disconnect?')
		)
			return;
		await backendState.disconnectDevice();
		if (!backendState.snapshot?.activeConnection) await goto('#/', { replaceState: true });
	}

	async function openConfigure() {
		menuOpen = false;
		configureOpen = true;
		await tick();
		sheetCloseButton?.focus();
	}

	async function closeConfigure() {
		if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
		await tick();
		await editor?.flush();
		configureOpen = false;
		await tick();
		menuButton?.focus();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (configureOpen && event.key === 'Escape') {
			event.preventDefault();
			void closeConfigure();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<section aria-labelledby="live-heading" class="space-y-5">
	{#if connection}
		<header class="relative flex items-start justify-between gap-4">
			<div>
				<p class="text-xs font-medium tracking-[0.2em] text-muted uppercase">Live board</p>
				<h2 id="live-heading" class="mt-1 text-2xl font-semibold">{connection.displayName}</h2>
				<p class:!text-success={live} class="mt-1 text-sm text-warning capitalize">
					{live ? 'Connected' : connection.state.replaceAll('_', ' ')}
				</p>
			</div>
			<button
				bind:this={menuButton}
				class="grid h-11 w-11 place-items-center rounded-full border border-outline bg-surface text-xl"
				aria-label="Board actions"
				aria-expanded={menuOpen}
				onclick={() => (menuOpen = !menuOpen)}>⋮</button
			>
			{#if menuOpen}
				<div
					class="absolute top-12 right-0 z-20 min-w-52 overflow-hidden rounded-xl border border-outline bg-elevated shadow-xl"
				>
					{#if connection.savedDeviceId}
						<button
							class="block w-full px-4 py-3 text-left hover:bg-surface"
							onclick={openConfigure}>Configure board</button
						>
					{/if}
					<button
						class="block w-full px-4 py-3 text-left text-danger hover:bg-surface"
						disabled={backendState.busy}
						onclick={disconnect}>Disconnect</button
					>
				</div>
			{/if}
		</header>

		{#if connection.firmware && !connection.firmware.supported}
			<div class="rounded-xl border border-warning/50 bg-warning/10 p-3 text-sm text-warning">
				Firmware {connection.firmware.vescVersion} / {connection.firmware.packageVersion} is not supported.
				Live values may be incomplete.
			</div>
		{/if}
		{#if savedDevice && !savedDevice.profileComplete}
			<div class="rounded-xl border border-warning/50 bg-warning/10 p-3 text-sm text-warning">
				This board has no complete Board Profile. Live monitoring remains available.
			</div>
		{/if}

		<div class:opacity-45={dimmed} class="space-y-5 transition-opacity">
			<TelemetrySection
				title="Ride"
				items={[
					['Ride state', sample?.rideState ?? '—'],
					['Board state', sample?.boardState?.toString() ?? '—'],
					['Session time', duration(sample?.elapsedMs)]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Electrical"
				items={[
					[
						'Pack voltage',
						sample?.packVoltageV == null ? '—' : `${number(sample.packVoltageV, 2)} V`
					],
					['Duty cycle', sample?.dutyCycle == null ? '—' : `${number(sample.dutyCycle * 100)}%`],
					[
						'Motor current',
						sample?.motorCurrentA == null ? '—' : `${number(sample.motorCurrentA)} A`
					],
					[
						'Battery current',
						sample?.batteryCurrentA == null ? '—' : `${number(sample.batteryCurrentA)} A`
					]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Motion"
				items={[
					['Speed', speed(sample?.speedKph)],
					['ERPM', sample?.erpm == null ? '—' : Math.round(sample.erpm).toLocaleString()]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Thermal and controls"
				items={[
					['Controller', temperature(sample?.controllerTempC)],
					['Motor', temperature(sample?.motorTempC)],
					['Footpad state', sample?.footpadState?.toString() ?? '—']
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Active session"
				items={[
					['Samples', session?.sampleCount.toLocaleString() ?? '—'],
					['Ride time', duration(session?.ridingElapsedMs)],
					['Total time', duration(session?.totalElapsedMs)],
					['Distance', distance(session?.distanceM)],
					['Max speed', session?.maxSpeedMps == null ? '—' : speed(session.maxSpeedMps * 3.6)],
					[
						'Max duty',
						session?.maxDutyCycle == null ? '—' : `${number(session.maxDutyCycle * 100)}%`
					]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Firmware"
				items={[
					['Hardware', connection.firmware?.hardwareName ?? '—'],
					['VESC', connection.firmware?.vescVersion ?? '—'],
					[
						'Package',
						connection.firmware
							? `${connection.firmware.packageName} ${connection.firmware.packageVersion}`
							: '—'
					]
				] as [string, string][]}
			/>
		</div>
	{:else}
		<p class="text-muted">Recovering active connection…</p>
	{/if}
	{#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>

{#if configureOpen && connection?.savedDeviceId}
	<div class="fixed inset-0 z-40 bg-black/55">
		<button
			class="absolute inset-0 h-full w-full cursor-default"
			aria-label="Close board configuration"
			onclick={closeConfigure}
		></button>
		<div
			role="dialog"
			aria-modal="true"
			aria-labelledby="configure-sheet-title"
			class="absolute inset-x-0 bottom-0 max-h-[92dvh] overflow-y-auto rounded-t-3xl border border-outline bg-surface p-5 pb-[max(1.25rem,env(safe-area-inset-bottom))] shadow-2xl"
		>
			<div class="mx-auto mb-4 h-1 w-12 rounded-full bg-outline"></div>
			<div class="mb-5 flex items-center justify-between">
				<h2 id="configure-sheet-title" class="text-xl font-semibold">Board configuration</h2>
				<button
					bind:this={sheetCloseButton}
					class="rounded-full border border-outline px-3 py-1.5"
					onclick={closeConfigure}>Done</button
				>
			</div>
			<BoardEditor
				bind:this={editor}
				savedDeviceId={connection.savedDeviceId}
				embedded
				onclose={closeConfigure}
			/>
		</div>
	</div>
{/if}
