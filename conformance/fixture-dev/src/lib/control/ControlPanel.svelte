<script lang="ts">
	import { onMount } from 'svelte';
	import {
		firmwareOptions,
		refloatOptions,
		topologyOptions,
		type FixtureConfiguration,
		type FixtureSnapshot
	} from '../contract.ts';
	import ActionButton from './ActionButton.svelte';
	import { canonicalCanId, controlAvailability } from './availability.ts';
	import { ControlState } from './state.svelte.ts';

	let { initial, token }: { initial: FixtureSnapshot; token: string } = $props();
	// svelte-ignore state_referenced_locally
	const initialSnapshot = $state.snapshot(initial);
	// svelte-ignore state_referenced_locally
	const initialToken = $state.snapshot(token);
	const control = new ControlState(initialSnapshot, initialToken);
	let selectedPort = $state('');
	let draft = $state({
		topology: initialSnapshot.configuration.topology,
		canId: String(initialSnapshot.configuration.canId),
		firmware: initialSnapshot.configuration.firmware,
		refloat: initialSnapshot.configuration.refloat
	});
	let lastSavedRevision = initialSnapshot.revision;

	const snapshot = $derived(control.snapshot);
	const available = $derived(controlAvailability(snapshot));
	const canId = $derived(canonicalCanId(draft.canId));
	const configurationValid = $derived(canId !== undefined);

	$effect(() => {
		if (snapshot.revision === lastSavedRevision) return;
		if (!available.configure) return;
		draft = {
			topology: snapshot.configuration.topology,
			canId: String(snapshot.configuration.canId),
			firmware: snapshot.configuration.firmware,
			refloat: snapshot.configuration.refloat
		};
		lastSavedRevision = snapshot.revision;
	});

	onMount(() => {
		void control.start();
		return () => control.stop();
	});

	function save() {
		if (canId === undefined) return;
		const configuration: FixtureConfiguration = {
			topology: draft.topology,
			canId,
			firmware: draft.firmware,
			refloat: draft.refloat
		};
		return control.act({
			kind: 'configure',
			configuration: { ...configuration, canId: draft.canId }
		});
	}
</script>

<svelte:head><title>Float View fixture</title></svelte:head>

<main>
	<header>
		<p class="eyebrow">Float View development fixture</p>
		<h1>Refloat board simulator</h1>
		<p>Drive the unchanged ESP32 byte bridge while a native Float View backend is running.</p>
	</header>

	{#if control.error}
		<p class="error" role="alert">{control.error}</p>
	{/if}

	<section aria-labelledby="serial-heading">
		<div class="section-heading">
			<h2 id="serial-heading">Serial fixture</h2>
			<span class:good={snapshot.serial.status === 'attached'}>{snapshot.serial.status}</span>
		</div>
		<div class="row">
			<select bind:value={selectedPort} disabled={!available.attach || control.loading}>
				<option value="">Select a serial port</option>
				{#each control.ports as port (port.path)}
					<option value={port.path}>{port.label}</option>
				{/each}
			</select>
			<ActionButton
				label="Attach"
				disabled={!available.attach || !selectedPort || control.loading}
				action={() => control.act({ kind: 'attach', path: selectedPort })}
				kind="primary"
			/>
			<ActionButton
				label="Detach"
				disabled={!available.detach || control.loading}
				action={() => control.act({ kind: 'detach' })}
			/>
		</div>
		{#if snapshot.serial.path}<p class="detail">{snapshot.serial.path}</p>{/if}
	</section>

	<section aria-labelledby="configuration-heading">
		<div class="section-heading">
			<h2 id="configuration-heading">Device configuration</h2>
			<span>THOR300</span>
		</div>
		<div class="form-grid">
			<label>
				Topology
				<select bind:value={draft.topology} disabled={!available.configure || control.loading}>
					{#each topologyOptions as topology}
						<option value={topology}>{topology === 'express' ? 'VESC Express → CAN' : 'Direct VESC'}</option>
					{/each}
				</select>
			</label>
			<label>
				CAN ID
				<input
					type="text"
					inputmode="numeric"
					autocomplete="off"
					bind:value={draft.canId}
					disabled={!available.configure || draft.topology === 'direct' || control.loading}
					aria-invalid={!configurationValid}
				/>
			</label>
			<label>
				VESC firmware
				<select bind:value={draft.firmware} disabled={!available.configure || control.loading}>
					{#each firmwareOptions as firmware}<option value={firmware}>{firmware}</option>{/each}
				</select>
			</label>
			<label>
				Refloat
				<select bind:value={draft.refloat} disabled={!available.configure || control.loading}>
					{#each refloatOptions as refloat}<option value={refloat}>{refloat}</option>{/each}
				</select>
			</label>
		</div>
		{#if !configurationValid}<p class="hint">CAN ID must be a canonical decimal integer from 0 to 253.</p>{/if}
		<ActionButton
			label="Save configuration"
			disabled={!available.configure || !configurationValid || control.loading}
			action={save}
			kind="primary"
		/>
	</section>

	<section aria-labelledby="power-heading">
		<div class="section-heading">
			<h2 id="power-heading">Board controls</h2>
			<span class:good={snapshot.ble === 'connected'}>{snapshot.power} · {snapshot.ble}</span>
		</div>
		<div class="row">
			<ActionButton
				label="Power on"
				disabled={!available.powerOn || control.loading}
				action={() => control.act({ kind: 'power-on' })}
				kind="primary"
			/>
			<ActionButton
				label="Power off"
				disabled={!available.powerOff || control.loading}
				action={() => control.act({ kind: 'power-off' })}
				kind="danger"
			/>
			<ActionButton
				label="Start riding"
				disabled={!available.startRide || control.loading}
				action={() => control.act({ kind: 'start-riding' })}
				kind="primary"
			/>
			<ActionButton
				label="Stop riding"
				disabled={!available.stopRide || control.loading}
				action={() => control.act({ kind: 'stop-riding' })}
			/>
		</div>
	</section>

	<section aria-labelledby="telemetry-heading">
		<div class="section-heading">
			<h2 id="telemetry-heading">Synthetic telemetry</h2>
			<span>{snapshot.ride}</span>
		</div>
		{#if snapshot.telemetry}
			<div class="metrics">
				<article><strong>{snapshot.telemetry.speedKph.toFixed(1)}</strong><span>km/h</span></article>
				<article><strong>{snapshot.telemetry.packVoltageV.toFixed(1)}</strong><span>volts</span></article>
				<article><strong>{snapshot.telemetry.motorCurrentA.toFixed(1)}</strong><span>motor amps</span></article>
				<article><strong>{snapshot.telemetry.controllerTempC.toFixed(1)}</strong><span>controller °C</span></article>
			</div>
		{:else}
			<p class="detail">Telemetry appears after the native backend requests a sample.</p>
		{/if}
	</section>

	<section aria-labelledby="diagnostics-heading">
		<div class="section-heading">
			<h2 id="diagnostics-heading">Requests and diagnostics</h2>
			<span>revision {snapshot.revision}</span>
		</div>
		<p class="detail">
			{snapshot.counters.writes} writes · {snapshot.counters.responses} responses ·
			{snapshot.counters.telemetry} telemetry · {snapshot.counters.dropped} dropped
		</p>
		<ul>
			{#each snapshot.diagnostics as diagnostic}<li>{diagnostic}</li>{/each}
		</ul>
	</section>
</main>

<style>
	:global(*) {
		box-sizing: border-box;
	}
	:global(body) {
		margin: 0;
		background: #07111f;
		color: #e2e8f0;
		font-family:
			Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
	}
	main {
		width: min(68rem, calc(100% - 2rem));
		margin: 0 auto;
		padding: 3rem 0 6rem;
	}
	header {
		margin-bottom: 1.5rem;
	}
	h1 {
		margin: 0.2rem 0 0.4rem;
		font-size: clamp(2rem, 5vw, 3.6rem);
		letter-spacing: -0.045em;
	}
	h2 {
		margin: 0;
		font-size: 1.05rem;
	}
	p {
		color: #94a3b8;
	}
	.eyebrow {
		margin: 0;
		color: #2dd4bf;
		font-size: 0.76rem;
		font-weight: 800;
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}
	section {
		margin-top: 1rem;
		border: 1px solid #243349;
		border-radius: 1rem;
		background: #0f1b2d;
		padding: 1.1rem;
	}
	.section-heading,
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		flex-wrap: wrap;
	}
	.section-heading {
		margin-bottom: 1rem;
	}
	.section-heading span {
		color: #94a3b8;
		font-size: 0.86rem;
		text-transform: capitalize;
	}
	.section-heading span.good {
		color: #5eead4;
	}
	.row {
		justify-content: flex-start;
	}
	.form-grid,
	.metrics {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: 0.8rem;
		margin-bottom: 1rem;
	}
	label {
		display: grid;
		gap: 0.4rem;
		color: #94a3b8;
		font-size: 0.8rem;
		font-weight: 700;
	}
	select,
	input {
		min-height: 2.7rem;
		border: 1px solid #475569;
		border-radius: 0.55rem;
		background: #07111f;
		color: #f8fafc;
		padding: 0.55rem 0.7rem;
		font: inherit;
	}
	select:disabled,
	input:disabled {
		opacity: 0.5;
	}
	input[aria-invalid="true"] {
		border-color: #fb7185;
	}
	.metrics article {
		display: grid;
		gap: 0.15rem;
		border-radius: 0.7rem;
		background: #07111f;
		padding: 0.9rem;
	}
	.metrics strong {
		font-size: 1.55rem;
	}
	.metrics span,
	.detail,
	.hint,
	li {
		color: #94a3b8;
		font-size: 0.82rem;
	}
	.error {
		border: 1px solid #fb7185;
		border-radius: 0.7rem;
		background: #4c0519;
		color: #fecdd3;
		padding: 0.8rem;
	}
	ul {
		max-height: 12rem;
		overflow: auto;
		margin: 0;
		padding-left: 1.2rem;
	}
	@media (max-width: 40rem) {
		main {
			width: min(100% - 1rem, 68rem);
			padding-top: 1.5rem;
		}
		.row :global(button) {
			flex: 1 1 9rem;
		}
	}
</style>
