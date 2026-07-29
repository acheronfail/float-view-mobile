<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { backendState } from '$lib/backend/state.svelte';

	let { sessionId }: { sessionId: string } = $props();

	function polyline(field: string): string {
		const values = backendState.sessionSeries
			.map((point) => ({ elapsedMs: point.elapsedMs, value: point.values[field] }))
			.filter(
				(point): point is { elapsedMs: number; value: number } => typeof point.value === 'number'
			);
		if (values.length < 2) return '';
		const first = values[0].elapsedMs;
		const elapsedRange = Math.max(1, values.at(-1)!.elapsedMs - first);
		const minimum = Math.min(...values.map((point) => point.value));
		const maximum = Math.max(...values.map((point) => point.value));
		const valueRange = Math.max(Number.EPSILON, maximum - minimum);
		return values
			.map(
				(point) =>
					`${((point.elapsedMs - first) / elapsedRange) * 600},${120 - ((point.value - minimum) / valueRange) * 110}`
			)
			.join(' ');
	}

	onMount(() => {
		let active = true;
		void backendState.openSession(sessionId).then((opened) => {
			if (active && !opened) void goto('#/sessions', { replaceState: true });
		});
		return () => {
			active = false;
			backendState.closeSession();
		};
	});

	async function inspectMidpoint() {
		const session = backendState.selectedSession;
		if (!session) return;
		await backendState.loadDiagnosticWindow(session.totalElapsedMs / 2);
		document.getElementById('diagnostic-window')?.scrollIntoView({ behavior: 'smooth' });
	}

	async function deleteSession() {
		const session = backendState.selectedSession;
		if (
			!session ||
			!window.confirm(
				`Permanently delete this ${session.sampleCount}-sample session? This cannot be undone.`
			)
		)
			return;
		if (await backendState.deleteSelectedSession())
			await goto('#/sessions', { replaceState: true });
	}
</script>

<section aria-labelledby="session-heading" class="rounded-2xl border border-outline bg-surface p-5">
	<a class="text-accent" href="#/sessions">← Sessions</a>
	{#if backendState.selectedSession}
		<h2 id="session-heading" class="mt-3 text-xl font-medium">Review</h2>
		<p class="mt-2 text-lg">{backendState.selectedSession.displayName}</p>
		<div class="mt-4 grid grid-cols-2 gap-3">
			<div class="rounded-lg bg-elevated p-3">
				<p class="text-sm text-muted">Samples</p>
				<p class="text-xl">{backendState.selectedSession.sampleCount}</p>
			</div>
			<div class="rounded-lg bg-elevated p-3">
				<p class="text-sm text-muted">Distance</p>
				<p class="text-xl">
					{((backendState.selectedSession.distanceM ?? 0) / 1000).toFixed(2)} km
				</p>
			</div>
		</div>

		<div class="mt-5 rounded-lg border border-outline p-4">
			<h3 class="font-medium">Ride trends</h3>
			{#if backendState.sessionSeries.length}
				<svg
					class="mt-3 h-40 w-full rounded bg-canvas"
					viewBox="0 0 600 130"
					role="img"
					aria-label="Voltage and duty cycle over the recorded session"
				>
					<polyline
						points={polyline('packVoltageV')}
						fill="none"
						stroke="var(--app-accent)"
						stroke-width="3"
					/>
					<polyline
						points={polyline('dutyCycle')}
						fill="none"
						stroke="var(--app-success)"
						stroke-width="2"
					/>
				</svg>
				<p class="mt-2 text-xs text-muted">{backendState.sessionSeries.length} bounded points</p>
			{:else}
				<p class="mt-2 text-sm text-muted">No samples were recorded for this session.</p>
			{/if}
			<button
				class="mt-3 rounded-md bg-elevated px-3 py-2 disabled:opacity-50"
				disabled={backendState.busy || backendState.selectedSession.totalElapsedMs === 0}
				onclick={inspectMidpoint}>Inspect midpoint ±15 seconds</button
			>
			{#if backendState.diagnosticSeries.length}
				<div id="diagnostic-window" class="mt-4 rounded bg-canvas p-3">
					<p class="text-sm font-medium">Diagnostic window</p>
					<p class="mt-1 text-xs text-muted">
						{backendState.diagnosticSeries.length} lossless samples
					</p>
				</div>
			{/if}
		</div>

		<div class="mt-5 rounded-lg border border-outline p-4">
			<h3 class="font-medium">Session file</h3>
			<p class="mt-1 text-sm text-muted">
				Exports open in the native platform save or share surface.
			</p>
			<div class="mt-3 flex flex-wrap gap-2">
				<button
					class="rounded-md bg-elevated px-3 py-2 disabled:opacity-50"
					disabled={backendState.busy || backendState.selectedSession.state === 'active'}
					onclick={() => backendState.exportSession('float-view-archive')}
					>Save/share archive</button
				>
				<button
					class="rounded-md bg-elevated px-3 py-2 disabled:opacity-50"
					disabled={backendState.busy || backendState.selectedSession.state === 'active'}
					onclick={() => backendState.exportSession('csv')}>Save/share CSV</button
				>
				<button
					class="rounded-md bg-danger px-3 py-2 text-white disabled:opacity-50"
					disabled={backendState.busy || backendState.selectedSession.state === 'active'}
					onclick={deleteSession}>Delete session</button
				>
			</div>
			{#if backendState.exportStatus}<p class="mt-3 text-sm text-success">
					{backendState.exportStatus}
				</p>{/if}
		</div>
	{:else}
		<p class="mt-3 text-muted">Loading session…</p>
	{/if}
	{#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>
