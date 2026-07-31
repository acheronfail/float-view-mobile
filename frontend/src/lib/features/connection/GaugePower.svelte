<script lang="ts">
	import type { LiveSample } from '../../backend/contract';

	let {
		sample
	}: {
		sample: LiveSample | null | undefined;
	} = $props();

	const green = [22, 163, 74];
	const yellow = [234, 179, 8];
	const red = [220, 38, 38];

	const batteryPercent = $derived(
		sample?.batteryLevel == null ? null : Math.min(Math.max(sample.batteryLevel * 100, 0), 100)
	);
	const batteryWidth = $derived(batteryPercent ?? 0);
	const charging = $derived(sample?.charging === true);
	const batteryColor = $derived(colorForBattery(batteryPercent));

	function interpolate(start: number[], end: number[], amount: number) {
		return start.map((value, index) => Math.round(value + (end[index] - value) * amount));
	}

	function rgb(color: number[]) {
		return `rgb(${color.join(', ')})`;
	}

	function colorForBattery(percent: number | null) {
		if (percent == null || percent >= 50) return rgb(green);
		if (percent <= 20) return rgb(red);
		if (percent < 35) return rgb(interpolate(yellow, red, (35 - percent) / 15));
		return rgb(interpolate(green, yellow, (50 - percent) / 15));
	}

	function voltage(value: number | null | undefined) {
		return value == null ? '—' : `${value.toFixed(2)} V`;
	}
</script>

<section
	class="rounded-2xl border border-outline bg-surface p-3 sm:p-5"
	aria-labelledby="power-title"
>
	<div class="mb-3 flex items-baseline justify-between gap-3">
		<h2 id="power-title" class="text-xs font-semibold tracking-[0.16em] text-muted uppercase">
			Battery
		</h2>
		<span class="font-mono text-sm font-semibold text-foreground tabular-nums">
			{batteryPercent == null ? '—' : `${batteryPercent.toFixed(0)}%`}
		</span>
	</div>

	<div class="flex items-center gap-3">
		<div
			class="relative h-5 min-w-0 flex-1 overflow-hidden rounded-full bg-elevated"
			role="progressbar"
			aria-label="Battery level"
			aria-valuemin="0"
			aria-valuemax="100"
			aria-valuenow={batteryPercent ?? undefined}
			aria-valuetext={batteryPercent == null
				? 'Unavailable'
				: `${batteryPercent.toFixed(0)} percent`}
		>
			<div
				class="pointer-events-none absolute inset-0 z-10 flex justify-between"
				aria-hidden="true"
			>
				{#each Array.from({ length: 11 }) as _, index}
					<span class="h-full w-px bg-foreground/20"></span>
				{/each}
			</div>
			<div
				class:charging-fill={charging}
				class="battery-fill relative h-full rounded-full transition-[width,background-color] duration-500"
				style={`width: ${batteryWidth}%; --battery-color: ${batteryColor}`}
			></div>
		</div>
		<output class="w-[5.5rem] shrink-0 text-right font-mono text-sm font-semibold tabular-nums">
			{voltage(sample?.packVoltageV)}
		</output>
	</div>

	<div
		class="mt-1 flex justify-between pr-[6.5rem] pl-0.5 text-[0.625rem] text-muted/70"
		aria-hidden="true"
	>
		{#each [0, 20, 40, 60, 80, 100] as tick}
			<span>{tick}%</span>
		{/each}
	</div>
	{#if charging}
		<p
			class="mt-2 text-right text-[0.625rem] font-semibold tracking-[0.14em] text-warning uppercase"
		>
			Charging
		</p>
	{/if}
</section>

<style>
	.battery-fill {
		background: linear-gradient(90deg, #16a34a 0%, var(--battery-color) 100%);
	}

	.charging-fill {
		background: linear-gradient(90deg, #ea580c, #f59e0b, #ea580c);
		background-size: 200% 100%;
		animation: charging-pulse 1.8s linear infinite;
	}

	@keyframes charging-pulse {
		from {
			background-position: 100% 0;
		}
		to {
			background-position: 0 0;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.charging-fill {
			animation: none;
		}
	}
</style>
