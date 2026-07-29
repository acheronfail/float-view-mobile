<script lang="ts">
	import type { SeriesPoint } from '$lib/backend/contract';
	import type { HistoricalField } from './telemetry';
	import { seriesPolyline, seriesValues } from './telemetry';

	let {
		title,
		fields,
		points
	}: { title: string; fields: readonly HistoricalField[]; points: readonly SeriesPoint[] } = $props();
</script>

<section class="rounded-lg border border-outline p-4" aria-label={`${title} historical telemetry`}>
	<h3 class="font-medium">{title}</h3>
	<div class="mt-3 space-y-4">
		{#each fields as field (field.key)}
			{@const values = seriesValues(points, field.key)}
			<div>
				<div class="flex items-baseline justify-between gap-3 text-sm">
					<span>{field.label} <span class="text-muted">({field.unit})</span></span>
					{#if values.length}
						<span class="font-mono text-xs">
							{values.at(-1)!.value.toLocaleString()} · {Math.min(
								...values.map((point) => point.value)
							).toLocaleString()}–{Math.max(
								...values.map((point) => point.value)
							).toLocaleString()}
						</span>
					{:else}
						<span class="text-xs text-muted">Unavailable</span>
					{/if}
				</div>
				{#if values.length}
					<svg
						class="mt-1 h-12 w-full rounded bg-canvas"
						viewBox="0 0 320 50"
						role="img"
						aria-label={`${field.label} over the recorded session in ${field.unit}`}
					>
						<polyline
							points={seriesPolyline(points, field.key)}
							fill="none"
							stroke="var(--app-accent)"
							stroke-width="2"
							vector-effect="non-scaling-stroke"
						/>
					</svg>
				{/if}
			</div>
		{/each}
	</div>
</section>
