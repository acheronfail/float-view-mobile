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
	import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
	import BottomSheet from '$lib/ui/BottomSheet.svelte';
	import ConfirmDialog from '$lib/ui/ConfirmDialog.svelte';
	import ControllerSelection from './ControllerSelection.svelte';
	import TelemetrySection from './TelemetrySection.svelte';
	import {
		controllerRouteDescription,
		firmwareAccessDescription,
		readingsAreDimmed,
		shouldConfirmDisconnect
	} from './live';
	import {
		currentTelemetryFields,
		extensionTelemetry,
		slowTelemetryAge,
		telemetryFlag,
		telemetryState
	} from './telemetry';
	import GaugePrimary from './GaugePrimary.svelte';

	let menuOpen = $state(false);
	let configureOpen = $state(false);
	let disconnectConfirmationOpen = $state(false);
	let disconnecting = $state(false);
	let menuButton = $state<HTMLButtonElement>();
	let editor = $state<BoardEditor>();
	const connection = $derived(backendState.snapshot?.activeConnection);
	const sample = $derived(connection?.latestSample);
	const session = $derived(connection?.activeSession);
	const units = $derived(backendState.settings?.unitPreference ?? 'metric');
	const live = $derived(connection?.state === 'ready');
	const dimmed = $derived(connection ? readingsAreDimmed(connection) : false);
	const routeDescription = $derived(
		controllerRouteDescription(connection?.controllerConnection ?? null)
	);
	const savedDevice = $derived(
		backendState.savedDevices.find((device) => device.savedDeviceId === connection?.savedDeviceId)
	);
	const safetyStateActive = $derived(
		Boolean(
			sample &&
				(sample.wheelslip ||
					(sample.stopCondition && sample.stopCondition !== 'none') ||
					(sample.setpointAdjustment && sample.setpointAdjustment !== 'none') ||
					(sample.alertReason && sample.alertReason !== 'none') ||
					sample.firmwareFault)
		)
	);

	$effect(() => {
		if (backendState.status === 'connected' && !connection) {
			void goto('#/', { replaceState: true });
		}
	});

	function number(value: number | null | undefined, digits = 1) {
		return value == null ? '—' : value.toFixed(digits);
	}

	function measured(value: number | null | undefined, unit: string, digits = 1) {
		return value == null ? 'Unavailable' : `${number(value, digits)} ${unit}`;
	}

	function scalar(value: number | null | undefined, digits = 1) {
		return value == null ? 'Unavailable' : number(value, digits);
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

	function requestDisconnect() {
		menuOpen = false;
		if (shouldConfirmDisconnect(sample?.rideState)) {
			disconnectConfirmationOpen = true;
			return;
		}
		void disconnect();
	}

	async function disconnect() {
		if (disconnecting) return;
		disconnecting = true;
		await backendState.disconnectDevice();
		disconnecting = false;
		disconnectConfirmationOpen = false;
		if (!backendState.snapshot?.activeConnection) await goto('#/', { replaceState: true });
	}

	async function cancelDisconnect() {
		disconnectConfirmationOpen = false;
		await tick();
		menuButton?.focus();
	}

	async function openConfigure() {
		menuOpen = false;
		configureOpen = true;
	}

	async function closeConfigure() {
		if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
		await tick();
		await editor?.flush();
		configureOpen = false;
		await tick();
		menuButton?.focus();
	}

	function toggleMenu() {
		menuOpen = !menuOpen;
	}
</script>

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
				type="button"
				class="grid h-11 w-11 touch-manipulation place-items-center rounded-full border border-outline bg-surface text-xl"
				aria-label="Board actions"
				aria-expanded={menuOpen}
				onpointerdown={(event) => activateFromPointer(event, toggleMenu)}
				onclick={(event) => activateFromKeyboardOrAssistiveClick(event, toggleMenu)}>⋮</button
			>
			{#if menuOpen}
				<div
					class="absolute top-12 right-0 z-20 min-w-52 overflow-hidden rounded-xl border border-outline bg-elevated shadow-xl"
				>
					{#if connection.savedDeviceId}
						<button
							type="button"
							class="block w-full touch-manipulation px-4 py-3 text-left hover:bg-surface"
							onpointerdown={(event) => activateFromPointer(event, openConfigure)}
							onclick={(event) => activateFromKeyboardOrAssistiveClick(event, openConfigure)}
							>Configure board</button
						>
					{/if}
					<button
						type="button"
						class="block w-full touch-manipulation px-4 py-3 text-left text-danger hover:bg-surface"
						disabled={backendState.busy}
						onpointerdown={(event) => activateFromPointer(event, requestDisconnect)}
						onclick={(event) => activateFromKeyboardOrAssistiveClick(event, requestDisconnect)}
						>Disconnect</button
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
		{#if connection.connectionIssue}
			<div class="rounded-xl border border-warning/50 bg-warning/10 p-3 text-sm text-warning">
				<p>{connection.connectionIssue.message}</p>
				{#if connection.state === 'failed'}
					<button
						type="button"
						class="mt-3 touch-manipulation rounded-md border border-warning/60 px-3 py-2 font-medium"
						disabled={backendState.busy}
						onpointerdown={(event) =>
							activateFromPointer(event, async () => {
								await backendState.connectDevice(connection.deviceId);
							})}
						onclick={(event) =>
							activateFromKeyboardOrAssistiveClick(event, async () => {
								await backendState.connectDevice(connection.deviceId);
							})}
					>
						{backendState.busy ? 'Retrying…' : 'Retry connection'}
					</button>
				{/if}
			</div>
		{/if}
		{#if savedDevice && !savedDevice.profileComplete}
			<button
				class="rounded-xl text-left border border-warning/50 bg-warning/10 p-3 text-sm text-warning"
				onpointerdown={(event) => activateFromPointer(event, openConfigure)}
				onclick={(event) => activateFromKeyboardOrAssistiveClick(event, openConfigure)}
			>
				This board has no complete Board Profile, tap to configure.
			</button>
		{/if}
		{#if safetyStateActive}
			<div role="status" class="rounded-xl border border-warning/50 bg-warning/10 p-3 text-sm">
				<strong>Reported board safety state is active.</strong>
				These readings describe onboard controller behavior; Float View does not control the board.
			</div>
		{/if}

		<div class:opacity-45={dimmed} class="space-y-5 transition-opacity">
			<GaugePrimary {sample} {units} />
		</div>

		<div class:opacity-45={dimmed} class="space-y-5 transition-opacity">
			<TelemetrySection
				title="Board state"
				items={[
					['Ride state', telemetryState(sample?.rideState)],
					['Package state', telemetryState(sample?.packageState)],
					['Package mode', telemetryState(sample?.packageMode)],
					['Footpad', telemetryState(sample?.footpadState)],
					['Wheelslip', telemetryFlag(sample?.wheelslip)],
					['Darkride', telemetryFlag(sample?.darkride)],
					['Charging', telemetryFlag(sample?.charging)],
					['Stop condition', telemetryState(sample?.stopCondition)],
					['Setpoint adjustment / pushback', telemetryState(sample?.setpointAdjustment)],
					['Alert reason', telemetryState(sample?.alertReason)],
					[
						'Active firmware fault',
						sample
							? sample.firmwareFault
								? telemetryState(sample.firmwareFault)
								: 'None'
							: 'Unavailable'
					],
					['Session time', duration(sample?.elapsedMs)]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Motion"
				items={[
					['Speed', speed(sample?.speedKph)],
					['ERPM', sample?.erpm == null ? 'Unavailable' : Math.round(sample.erpm).toLocaleString()],
					['Pitch', measured(sample?.pitchDeg, '°')],
					['Balance pitch', measured(sample?.balancePitchDeg, '°')],
					['Roll', measured(sample?.rollDeg, '°')]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Raw electrical load"
				items={[
					['Pack voltage', measured(sample?.packVoltageV, 'V', 2)],
					['Duty ratio', scalar(sample?.dutyCycle, 3)],
					...currentTelemetryFields.map(({ key, label }) => [
						label,
						measured(sample?.[key] as number | null | undefined, 'A')
					])
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Temperatures"
				items={[
					['Controller', temperature(sample?.controllerTempC)],
					['Motor', temperature(sample?.motorTempC)]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Rider input"
				items={[
					['Footpad ADC 1', measured(sample?.footpadAdc1V, 'V', 2)],
					['Footpad ADC 2', measured(sample?.footpadAdc2V, 'V', 2)],
					['Remote input', scalar(sample?.remoteInput, 3)]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Control contributions"
				items={[
					['Final setpoint', measured(sample?.setpointDeg, '°')],
					['ATR setpoint', measured(sample?.atrSetpointDeg, '°')],
					['Brake-tilt setpoint', measured(sample?.brakeTiltSetpointDeg, '°')],
					['Torque-tilt setpoint', measured(sample?.torqueTiltSetpointDeg, '°')],
					['Turn-tilt setpoint', measured(sample?.turnTiltSetpointDeg, '°')],
					['Remote setpoint', measured(sample?.remoteSetpointDeg, '°')],
					[
						'ATR acceleration difference',
						measured(sample?.atrAccelerationDifferenceErpmPerSecond, 'ERPM/s', 0)
					],
					['ATR speed boost', scalar(sample?.atrSpeedBoost, 3)]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Accumulated and charging"
				items={[
					['Trip distance', distance(sample?.distanceM)],
					['Odometer', distance(sample?.odometerM)],
					['Amp-hours drawn', measured(sample?.ampHoursDrawnAh, 'Ah', 2)],
					['Amp-hours charged', measured(sample?.ampHoursChargedAh, 'Ah', 2)],
					['Watt-hours drawn', measured(sample?.wattHoursDrawnWh, 'Wh', 1)],
					['Watt-hours charged', measured(sample?.wattHoursChargedWh, 'Wh', 1)],
					[
						'Battery level',
						sample?.batteryLevel == null
							? 'Unavailable'
							: `${number(sample.batteryLevel * 100, 0)}%`
					],
					['Charging current', measured(sample?.chargingCurrentA, 'A')],
					['Charging voltage', measured(sample?.chargingVoltageV, 'V', 2)]
				] as [string, string][]}
			/>
			<TelemetrySection
				title="Source availability"
				items={[
					['Realtime sample', sample?.observedAt ?? 'Unavailable'],
					['Compact values', slowTelemetryAge(sample)],
					[
						'Negotiated extensions',
						sample ? Object.keys(sample.extensions).length.toString() : 'Unavailable'
					],
					...extensionTelemetry(sample)
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
					['Connection', routeDescription],
					['Hardware', connection.firmware?.hardwareName ?? '—'],
					['VESC', connection.firmware?.vescVersion ?? '—'],
					[
						'Package',
						connection.firmware
							? `${connection.firmware.packageName} ${connection.firmware.packageVersion}`
							: '—'
					],
					[
						'Access',
						firmwareAccessDescription(connection.firmware)
					]
				] as [string, string][]}
			/>
		</div>
	{:else}
		<p class="text-muted">Recovering active connection…</p>
	{/if}
	{#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>

<ConfirmDialog
	open={disconnectConfirmationOpen}
	titleId="disconnect-confirmation-title"
	title="Disconnect board?"
	description="The board is not idle. This will stop the active session and disconnect the board."
	confirmLabel="Disconnect"
	destructive
	busy={disconnecting}
	onconfirm={disconnect}
	oncancel={cancelDisconnect}
/>

{#if configureOpen && connection?.savedDeviceId}
	<BottomSheet title="Board configuration" titleId="configure-sheet-title" onclose={closeConfigure}>
		<BoardEditor
			bind:this={editor}
			savedDeviceId={connection.savedDeviceId}
			embedded
			onclose={closeConfigure}
		/>
	</BottomSheet>
{/if}

{#if connection?.state === 'selecting_controller'}
	<ControllerSelection {connection} ondisconnect={disconnect} />
{/if}
