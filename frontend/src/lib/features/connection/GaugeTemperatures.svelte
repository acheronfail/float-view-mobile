<script lang="ts">
	import type { LiveSample } from '../../backend/contract';

	let {
		sample
	}: {
		sample: LiveSample | null | undefined;
	} = $props();

	const green = [22, 163, 74];
	const orange = [234, 88, 12];
	const red = [220, 38, 38];
	const maximum = 70;

	function interpolate(start: number[], end: number[], amount: number) {
		return start.map((value, index) => Math.round(value + (end[index] - value) * amount));
	}

	function rgb(color: number[]) {
		return `rgb(${color.join(', ')})`;
	}

	function colorForTemperature(value: number | null | undefined) {
		if (value == null || value <= 30) return rgb(green);
		if (value >= 60) return rgb(red);
		if (value >= 45) return rgb(interpolate(orange, red, (value - 45) / 15));
		return rgb(interpolate(green, orange, (value - 30) / 15));
	}

	function progress(value: number | null | undefined) {
		return value == null ? 0 : (Math.min(Math.max(value, 0), maximum) / maximum) * 100;
	}

	function label(value: number | null | undefined) {
		return value == null ? '—' : `${value.toFixed(1)} °C`;
	}

	const readings = $derived([
		{ key: 'controller', name: 'Controller', value: sample?.controllerTempC },
		{ key: 'motor', name: 'Motor', value: sample?.motorTempC }
	]);
</script>

<section
	class="rounded-2xl border border-outline bg-surface p-3 sm:p-5"
	aria-labelledby="temperatures-title"
>
	<h2
		id="temperatures-title"
		class="mb-3 text-xs font-semibold tracking-[0.16em] text-muted uppercase"
	>
		Temperatures
	</h2>

	<div class="space-y-3">
		{#each readings as reading (reading.key)}
			<div class="flex items-center gap-3">
				<span class="w-[4.75rem] shrink-0 text-sm text-muted">{reading.name}</span>
				<div
					class="relative h-4 min-w-0 flex-1 overflow-hidden rounded-full bg-elevated"
					role="progressbar"
					aria-label={`${reading.name} temperature`}
					aria-valuemin="0"
					aria-valuemax={maximum}
					aria-valuenow={reading.value ?? undefined}
					aria-valuetext={reading.value == null ? 'Unavailable' : label(reading.value)}
				>
					<div
						class="pointer-events-none absolute inset-0 z-10 flex justify-between"
						aria-hidden="true"
					>
						{#each Array.from({ length: 15 }) as _, index}
							<span class="h-full w-px bg-foreground/20"></span>
						{/each}
					</div>
					<div
						class="temperature-fill h-full rounded-full transition-[width,background-color] duration-500"
						style={`width: ${progress(reading.value)}%; --temperature-color: ${colorForTemperature(reading.value)}`}
					></div>
				</div>
				<output class="w-[5.5rem] shrink-0 text-right font-mono text-sm font-semibold tabular-nums">
					{label(reading.value)}
				</output>
			</div>
		{/each}
	</div>

	<div
		class="mt-1 flex justify-between pr-[6.5rem] pl-[4.75rem] text-[0.625rem] text-muted/70"
		aria-hidden="true"
	>
		{#each [0, 10, 20, 30, 40, 50, 60, 70] as tick}
			<span>{tick}°</span>
		{/each}
	</div>
</section>

<style>
	.temperature-fill {
		background: linear-gradient(90deg, #16a34a 0%, var(--temperature-color) 100%);
	}
</style>
