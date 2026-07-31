<script lang="ts">
	import { onMount } from 'svelte';
	import {
		idleTelemetry,
		telemetryFieldNames,
		telemetryScenes,
		type TelemetrySample
	} from '../../../../fixture/telemetry.ts';
	import {
		firmwareOptions,
		refloatOptions,
		scenarioTemplateOptions,
		topologyOptions,
		type FixtureConfiguration,
		type FixtureSnapshot,
		type ScenarioTemplate
	} from '../contract.ts';
	import ActionButton from './ActionButton.svelte';
	import { canonicalCanId, controlAvailability } from './availability.ts';
	import { ControlState } from './state.svelte.ts';

	let { initial, token }: { initial: FixtureSnapshot; token: string } = $props();
	// svelte-ignore state_referenced_locally
	const initialSnapshot = $state.snapshot(initial);
	// svelte-ignore state_referenced_locally
	const control = new ControlState(initialSnapshot, $state.snapshot(token));
	let selectedPort = $state('');
	let draft = $state({
		topology: initialSnapshot.configuration.topology,
		canId: String(initialSnapshot.configuration.canId),
		firmware: initialSnapshot.configuration.firmware,
		refloat: initialSnapshot.configuration.refloat
	});
	let selectedTemplate = $state<ScenarioTemplate>('singleExpress');
	let selectedScene = $state(initialSnapshot.scenario.scene);
	let scenarioText = $state(JSON.stringify(initialSnapshot.scenario, null, 2));
	let pinField = $state<keyof TelemetrySample>('controllerTempC');
	let pinValue = $state('77');
	let fragmentBytes = $state(String(initialSnapshot.scenario.fragmentBytes));
	let moveCanIds = $state<Record<string, string>>(
		Object.fromEntries(initialSnapshot.scenario.controllers.map((item) => [item.alias, String(item.canId)]))
	);
	let lastSavedRevision = initialSnapshot.revision;

	const snapshot = $derived(control.snapshot);
	const available = $derived(controlAvailability(snapshot));
	const canId = $derived(canonicalCanId(draft.canId));
	const configurationValid = $derived(canId !== undefined);
	const unavailableFields = $derived(
		telemetryFieldNames.filter((field) => !snapshot.availableFields.includes(field))
	);

	$effect(() => {
		if (snapshot.revision === lastSavedRevision || !available.configure) return;
		draft = {
			topology: snapshot.configuration.topology,
			canId: String(snapshot.configuration.canId),
			firmware: snapshot.configuration.firmware,
			refloat: snapshot.configuration.refloat
		};
		selectedScene = snapshot.scenario.scene;
		fragmentBytes = String(snapshot.scenario.fragmentBytes);
		moveCanIds = Object.fromEntries(
			snapshot.scenario.controllers.map((item) => [item.alias, String(item.canId)])
		);
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
			configuration
		});
	}

	function importScenario() {
		try {
			return control.act({ kind: 'import-scenario', document: JSON.parse(scenarioText) });
		} catch (error) {
			control.error = error instanceof Error ? error.message : String(error);
		}
	}

	function exportScenario() {
		scenarioText = JSON.stringify(snapshot.scenario, null, 2);
	}

	function pin() {
		const baseline = idleTelemetry[pinField];
		let value: unknown = pinValue;
		if (typeof baseline === 'number') value = Number(pinValue);
		else if (typeof baseline === 'boolean') value = pinValue === 'true';
		else if (pinField === 'firmwareFault' && pinValue === 'null') value = null;
		return control.act({ kind: 'pin-field', field: pinField, value } as Record<string, unknown>);
	}
</script>

<svelte:head><title>Float View telemetry workbench</title></svelte:head>

<main>
	<header>
		<p class="eyebrow">Float View development fixture</p>
		<h1>Refloat telemetry workbench</h1>
		<p>Compose a source-faithful board, then operate the native app manually. This surface never drives or judges the product.</p>
	</header>

	{#if control.error}<p class="error" role="alert">{control.error}</p>{/if}

	<section aria-labelledby="serial-heading">
		<div class="section-heading">
			<h2 id="serial-heading">Serial bridge</h2>
			<span class:good={snapshot.serial.status === 'attached'}>{snapshot.serial.status}</span>
		</div>
		<div class="row">
			<select aria-label="Serial port" bind:value={selectedPort} disabled={!available.attach || control.loading}>
				<option value="">Select a serial port</option>
				{#each control.ports as port (port.path)}<option value={port.path}>{port.label}</option>{/each}
			</select>
			<ActionButton label="Attach" disabled={!available.attach || !selectedPort || control.loading} action={() => control.act({ kind: 'attach', path: selectedPort })} kind="primary" />
			<ActionButton label="Detach" disabled={!available.detach || control.loading} action={() => control.act({ kind: 'detach' })} />
		</div>
		{#if snapshot.serial.path}<p class="detail">{snapshot.serial.path}</p>{/if}
		{#if snapshot.serial.error}<p class="error" role="alert">Serial bridge: {snapshot.serial.error}</p>{/if}
	</section>

	<section aria-labelledby="topology-heading">
		<div class="section-heading">
			<h2 id="topology-heading">Topology and scenario</h2>
			<span>{snapshot.scenario.topology} · {snapshot.scenario.controllers.length} controller{snapshot.scenario.controllers.length === 1 ? '' : 's'}</span>
		</div>
		<div class="row">
			<select aria-label="Scenario template" bind:value={selectedTemplate} disabled={!available.configure || control.loading}>
				{#each scenarioTemplateOptions as template}<option value={template}>{template}</option>{/each}
			</select>
			<ActionButton label="Load editable template" disabled={!available.configure || control.loading} action={() => control.act({ kind: 'apply-template', template: selectedTemplate })} />
		</div>
		<div class="form-grid compact">
			<label>Topology
				<select bind:value={draft.topology} disabled={!available.configure || control.loading}>
					{#each topologyOptions as topology}<option value={topology}>{topology}</option>{/each}
				</select>
			</label>
			<label>Primary CAN ID
				<input type="text" inputmode="numeric" bind:value={draft.canId} disabled={!available.configure || draft.topology === 'direct' || control.loading} aria-invalid={!configurationValid} />
			</label>
			<label>VESC firmware
				<select bind:value={draft.firmware} disabled={!available.configure || control.loading}>
					{#each firmwareOptions as firmware}<option value={firmware}>{firmware}</option>{/each}
				</select>
			</label>
			<label>Refloat
				<select bind:value={draft.refloat} disabled={!available.configure || control.loading}>
					{#each refloatOptions as refloat}<option value={refloat}>{refloat}</option>{/each}
				</select>
			</label>
		</div>
		<ActionButton label="Save common configuration" disabled={!available.configure || !configurationValid || control.loading} action={save} kind="primary" />

		<div class="candidate-list">
			{#each snapshot.scenario.controllers as candidate (candidate.alias)}
				<article>
					<div><strong>{candidate.alias}</strong><span>VESC {candidate.firmware} · Refloat {candidate.refloat}</span></div>
					<label>CAN ID
						<input type="text" inputmode="numeric" value={moveCanIds[candidate.alias] ?? String(candidate.canId)} oninput={(event) => (moveCanIds[candidate.alias] = event.currentTarget.value)} disabled={snapshot.scenario.topology === 'direct' || control.loading} />
					</label>
					<ActionButton label="Move" disabled={snapshot.scenario.topology === 'direct' || canonicalCanId(moveCanIds[candidate.alias] ?? '') === undefined || control.loading} action={() => control.act({ kind: 'move-controller', alias: candidate.alias, canId: canonicalCanId(moveCanIds[candidate.alias] ?? '')! })} />
					<ActionButton label={candidate.pingVisible ? 'Hide from ping' : 'Show in ping'} disabled={snapshot.scenario.topology === 'direct' || control.loading} action={() => control.act({ kind: 'set-ping-visible', alias: candidate.alias, visible: !candidate.pingVisible })} />
					<ActionButton label="Replace identity" disabled={control.loading || snapshot.power === 'on' && snapshot.serial.status !== 'attached'} action={() => control.act({ kind: 'replace-controller', alias: candidate.alias })} kind="danger" />
				</article>
			{/each}
		</div>
	</section>

	<section aria-labelledby="document-heading">
		<div class="section-heading">
			<h2 id="document-heading">Shareable scenario document</h2>
			<span>version 1 · 65,536-byte maximum</span>
		</div>
		<textarea aria-label="Scenario JSON" bind:value={scenarioText} spellcheck="false"></textarea>
		<div class="row">
			<ActionButton label="Refresh export" action={exportScenario} disabled={control.loading} />
			<ActionButton label="Import atomically" action={importScenario} disabled={!available.configure || control.loading} kind="primary" />
		</div>
		<p class="hint">Documents contain public aliases only—never serial paths, launch tokens, Bluetooth IDs, or private controller UUIDs.</p>
	</section>

	<section aria-labelledby="power-heading">
		<div class="section-heading">
			<h2 id="power-heading">Connection events</h2>
			<span class:good={snapshot.ble === 'connected'}>{snapshot.power} · {snapshot.ble}</span>
		</div>
		<div class="row">
			<ActionButton label="Power on" disabled={!available.powerOn || control.loading} action={() => control.act({ kind: 'power-on' })} kind="primary" />
			<ActionButton label="Power off" disabled={!available.powerOff || control.loading} action={() => control.act({ kind: 'power-off' })} kind="danger" />
			<ActionButton label="Disconnect / re-advertise" disabled={snapshot.power !== 'on' || snapshot.serial.status !== 'attached' || control.loading} action={() => control.act({ kind: 'disconnect-readvertise' })} />
			<ActionButton label="Reconnect unchanged" disabled={snapshot.power !== 'on' || snapshot.serial.status !== 'attached' || control.loading} action={() => control.act({ kind: 'reconnect-unchanged' })} />
		</div>
		<div class="row">
			<label>Notification fragment bytes
				<input type="number" min="1" max="244" bind:value={fragmentBytes} />
			</label>
			<ActionButton label="Apply fragmentation" disabled={Number(fragmentBytes) < 1 || Number(fragmentBytes) > 244 || control.loading} action={() => control.act({ kind: 'set-fragment-bytes', fragmentBytes: Number(fragmentBytes) })} />
		</div>
	</section>

	<section aria-labelledby="telemetry-heading">
		<div class="section-heading">
			<h2 id="telemetry-heading">Semantic telemetry scene</h2>
			<span>{snapshot.scenario.scene} · {snapshot.frozen ? 'frozen' : 'moving'}</span>
		</div>
		<div class="row">
			<select aria-label="Telemetry scene" bind:value={selectedScene}>
				{#each telemetryScenes as scene}<option value={scene}>{scene}</option>{/each}
			</select>
			<ActionButton label="Apply scene" disabled={control.loading} action={() => control.act({ kind: 'set-scene', scene: selectedScene })} kind="primary" />
			<ActionButton label={snapshot.frozen ? 'Resume motion' : 'Freeze motion'} disabled={control.loading} action={() => control.act({ kind: snapshot.frozen ? 'resume' : 'freeze' })} />
			<ActionButton label="Restore scene defaults" disabled={control.loading} action={() => control.act({ kind: 'reset-overrides' })} />
		</div>
		<div class="row">
			<label>Field override
				<select bind:value={pinField}>{#each telemetryFieldNames as field}<option value={field}>{field}</option>{/each}</select>
			</label>
			<label>Value
				<input type="text" bind:value={pinValue} />
			</label>
			<ActionButton label="Pin field" disabled={control.loading} action={pin} />
			<ActionButton label="Unpin field" disabled={!Object.hasOwn(snapshot.pinnedFields, pinField) || control.loading} action={() => control.act({ kind: 'unpin-field', field: pinField })} />
		</div>
		<div class="row">
			<ActionButton label="Start riding" disabled={!available.startRide || control.loading} action={() => control.act({ kind: 'start-riding' })} />
			<ActionButton label="Stop riding" disabled={!available.stopRide || control.loading} action={() => control.act({ kind: 'stop-riding' })} />
		</div>
		{#if snapshot.telemetry}
			<div class="metrics">
				<article><strong>{snapshot.telemetry.speedKph.toFixed(1)}</strong><span>km/h</span></article>
				<article><strong>{snapshot.telemetry.packVoltageV.toFixed(1)}</strong><span>volts</span></article>
				<article><strong>{snapshot.telemetry.motorCurrentA.toFixed(1)}</strong><span>actual motor A</span></article>
				<article><strong>{snapshot.telemetry.requestedBalanceCurrentA.toFixed(1)}</strong><span>requested A</span></article>
				<article><strong>{snapshot.telemetry.fieldWeakeningCurrentA.toFixed(1)}</strong><span>field weakening A</span></article>
				<article><strong>{snapshot.telemetry.controllerTempC.toFixed(1)}</strong><span>controller °C</span></article>
			</div>
		{/if}
		<p class="detail">{Object.keys(snapshot.pinnedFields).length} pinned · {snapshot.availableFields.length} source-available · {unavailableFields.length} unavailable</p>
		{#if unavailableFields.length}<p class="hint">Unavailable in every selected source: {unavailableFields.join(', ')}</p>{/if}
	</section>

	<section aria-labelledby="diagnostics-heading">
		<div class="section-heading">
			<h2 id="diagnostics-heading">Requests and diagnostics</h2>
			<span>revision {snapshot.revision}</span>
		</div>
		<p class="detail">{snapshot.counters.writes} writes · {snapshot.counters.responses} responses · {snapshot.counters.telemetry} telemetry · {snapshot.counters.dropped} dropped</p>
		{#if snapshot.lastAttempt}
			<p class:attempt-error={snapshot.lastAttempt.outcome === 'dropped' || snapshot.lastAttempt.outcome === 'failed'} class="attempt">
				Last attempt · command {snapshot.lastAttempt.command}{snapshot.lastAttempt.canId === undefined ? ' · direct' : ` · CAN ${snapshot.lastAttempt.canId}`} · {snapshot.lastAttempt.stage} · {snapshot.lastAttempt.outcome}{snapshot.lastAttempt.error ? ` · ${snapshot.lastAttempt.error}` : ''}
			</p>
		{/if}
		{#if snapshot.lastRequest}
			<p class="request">{snapshot.lastRequest.kind} · command {snapshot.lastRequest.command}{snapshot.lastRequest.canId === undefined ? '' : ` · CAN ${snapshot.lastRequest.canId}`} · {snapshot.lastRequest.responseBytes} bytes / {snapshot.lastRequest.fragments} fragment(s)</p>
		{/if}
		<ul>{#each snapshot.diagnostics as diagnostic}<li>{diagnostic}</li>{/each}</ul>
	</section>
</main>

<style>
	:global(*) { box-sizing: border-box; }
	:global(body) { margin: 0; background: #07111f; color: #e2e8f0; font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
	main { width: min(76rem, calc(100% - 2rem)); margin: 0 auto; padding: 3rem 0 6rem; }
	header { margin-bottom: 1.5rem; }
	h1 { margin: .2rem 0 .4rem; font-size: clamp(2rem, 5vw, 3.6rem); letter-spacing: -.045em; }
	h2 { margin: 0; font-size: 1.05rem; }
	p { color: #94a3b8; }
	.eyebrow { margin: 0; color: #2dd4bf; font-size: .76rem; font-weight: 800; letter-spacing: .12em; text-transform: uppercase; }
	section { margin-top: 1rem; border: 1px solid #243349; border-radius: 1rem; background: #0f1b2d; padding: 1.1rem; }
	.section-heading, .row { display: flex; align-items: center; justify-content: space-between; gap: .75rem; flex-wrap: wrap; }
	.section-heading { margin-bottom: 1rem; }
	.section-heading span { color: #94a3b8; font-size: .86rem; }
	.section-heading span.good { color: #5eead4; }
	.row { justify-content: flex-start; margin-top: .8rem; }
	.form-grid, .metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr)); gap: .8rem; margin: 1rem 0; }
	.form-grid.compact { grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr)); }
	label { display: grid; gap: .35rem; color: #94a3b8; font-size: .8rem; font-weight: 700; }
	select, input, textarea { min-height: 2.7rem; border: 1px solid #475569; border-radius: .55rem; background: #07111f; color: #f8fafc; padding: .55rem .7rem; font: inherit; }
	textarea { width: 100%; min-height: 14rem; resize: vertical; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: .76rem; }
	:disabled { opacity: .5; }
	[aria-invalid="true"] { border-color: #fb7185; }
	.candidate-list { display: grid; gap: .65rem; margin-top: 1rem; }
	.candidate-list article { display: flex; align-items: end; gap: .7rem; flex-wrap: wrap; border: 1px solid #26364c; border-radius: .7rem; padding: .75rem; }
	.candidate-list article > div { display: grid; min-width: 11rem; margin-right: auto; }
	.candidate-list span, .detail, .hint, li { color: #94a3b8; font-size: .82rem; }
	.metrics article { display: grid; gap: .15rem; border-radius: .7rem; background: #07111f; padding: .9rem; }
	.metrics strong { font-size: 1.45rem; }
	.metrics span { color: #94a3b8; font-size: .75rem; }
	.error { border: 1px solid #fb7185; border-radius: .7rem; background: #4c0519; color: #fecdd3; padding: .8rem; }
	.attempt { border-left: 3px solid #38bdf8; padding-left: .75rem; color: #cbd5e1; }
	.attempt-error { border-left-color: #fb7185; color: #fecdd3; }
	.request { border-left: 3px solid #2dd4bf; padding-left: .75rem; color: #cbd5e1; }
	ul { max-height: 12rem; overflow: auto; margin: 0; padding-left: 1.2rem; }
	@media (max-width: 40rem) {
		main { width: min(100% - 1rem, 76rem); padding-top: 1.5rem; }
		.row :global(button) { flex: 1 1 9rem; }
	}
</style>
