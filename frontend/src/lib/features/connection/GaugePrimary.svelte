<script lang="ts">
	import type { LiveSample, UnitPreference } from '../../backend/contract';
	import { formatDistanceM, formatSpeedKph } from '../measurements/format';

	let { sample, units }: { sample: LiveSample | null | undefined; units: UnitPreference } =
		$props();

	const center = 160;
	const outerRadius = 124;
	const innerRadius = 112;
	const startAngle = 135;
	const endAngle = 405;

	const speedMaximum = $derived(units === 'metric' ? 60 : 40);
	const speedUnit = $derived(units === 'metric' ? 'km/h' : 'mph');
	const speed = $derived(
		sample?.speedKph == null ? null : sample.speedKph * (units === 'metric' ? 1 : 0.621371)
	);
	const duty = $derived(sample?.dutyCycle == null ? null : Math.abs(sample.dutyCycle) * 100);

	function point(radius: number, angle: number) {
		const radians = (angle * Math.PI) / 180;
		return { x: center + radius * Math.cos(radians), y: center + radius * Math.sin(radians) };
	}

	function arc(radius: number) {
		const start = point(radius, startAngle);
		const end = point(radius, endAngle);
		return `M ${start.x} ${start.y} A ${radius} ${radius} 0 1 1 ${end.x} ${end.y}`;
	}

	function ticks(
		maximum: number,
		step: number,
		outerTickRadius: number,
		innerTickRadius: number,
		labelRadius: number
	) {
		return Array.from({ length: maximum / step + 1 }, (_, index) => {
			const value = index * step;
			const angle = startAngle + (value / maximum) * (endAngle - startAngle);
			return {
				value,
				outer: point(outerTickRadius, angle),
				inner: point(innerTickRadius, angle),
				label: point(labelRadius, angle)
			};
		});
	}

	const speedTicks = $derived(
		ticks(speedMaximum, 5, outerRadius + 9, outerRadius + 17, outerRadius + 29)
	);
	const dutyTicks = ticks(100, 10, innerRadius - 6, innerRadius - 14, innerRadius - 24);

	function progress(value: number | null, maximum: number) {
		return value == null ? 0 : (Math.min(Math.max(value, 0), maximum) / maximum) * 100;
	}
</script>

<div class="rounded-2xl border border-outline bg-surface p-3 sm:p-5">
	<svg
		viewBox="0 0 320 300"
		class="mx-auto block w-full max-w-md overflow-visible text-foreground"
		role="img"
		aria-labelledby="gauge-title gauge-description"
	>
		<title id="gauge-title">Live speed and duty gauge</title>
		<desc id="gauge-description">
			Speed is {formatSpeedKph(sample?.speedKph, units)} and duty cycle is {duty == null
				? 'unavailable'
				: `${duty.toFixed(0)} percent`}.
		</desc>

		<path
			d={arc(outerRadius)}
			pathLength="100"
			class="fill-none stroke-outline [stroke-width:15] opacity-55 [stroke-linecap:round]"
		/>
		<path
			d={arc(outerRadius)}
			pathLength="100"
			class="fill-none stroke-accent [stroke-width:15] [stroke-linecap:round]"
			stroke-dasharray={`${progress(speed, speedMaximum)} 100`}
		/>
		<path
			d={arc(innerRadius)}
			pathLength="100"
			class="fill-none stroke-outline [stroke-width:9] opacity-55 [stroke-linecap:round]"
		/>
		<path
			d={arc(innerRadius)}
			pathLength="100"
			class="fill-none stroke-success [stroke-width:9] [stroke-linecap:round]"
			stroke-dasharray={`${progress(duty, 100)} 100`}
		/>

		<g class="text-muted/40" aria-hidden="true">
			{#each speedTicks as tick (tick.value)}
				<line
					class="stroke-current [stroke-width:1.5]"
					x1={tick.outer.x}
					y1={tick.outer.y}
					x2={tick.inner.x}
					y2={tick.inner.y}
				/>
				<text
					class="fill-current text-[10px] font-semibold"
					x={tick.label.x}
					y={tick.label.y}
					text-anchor="middle"
					dominant-baseline="middle"
				>
					{tick.value}
				</text>
			{/each}
		</g>
		<g class="text-muted/40" aria-hidden="true">
			{#each dutyTicks as tick (tick.value)}
				<line
					class="stroke-current [stroke-width:1.5]"
					x1={tick.outer.x}
					y1={tick.outer.y}
					x2={tick.inner.x}
					y2={tick.inner.y}
				/>
				<text
					class="fill-current text-[9px]"
					x={tick.label.x}
					y={tick.label.y}
					text-anchor="middle"
					dominant-baseline="middle"
				>
					{tick.value}%
				</text>
			{/each}
		</g>

		<text
			x="160"
			y="116"
			class="fill-current text-[9px] font-semibold tracking-[0.12em] text-muted"
			text-anchor="middle">{speedUnit}</text
		>
		<text
			x="160"
			y="141"
			class="fill-current font-mono text-[25px] font-bold text-foreground tabular-nums"
			text-anchor="middle"
		>
			{speed == null ? '—' : speed.toFixed(1)}
		</text>
		<text
			x="160"
			y="163"
			class="fill-current text-[9px] font-semibold tracking-[0.12em] text-muted"
			text-anchor="middle"
		>
			Duty Cycle
		</text>
		<text
			x="160"
			y="188"
			class="fill-current font-mono text-[25px] font-bold text-foreground tabular-nums"
			text-anchor="middle"
		>
			{duty == null ? '—' : `${duty.toFixed(0)}%`}
		</text>

		<rect
			x="105"
			y="236"
			width="110"
			height="38"
			rx="9"
			class="fill-elevated stroke-outline [stroke-width:1]"
		/>
		<text
			x="160"
			y="251"
			class="fill-current text-[9px] font-semibold tracking-[0.12em] text-muted"
			text-anchor="middle">DISTANCE</text
		>
		<text
			x="160"
			y="268"
			class="fill-current font-mono text-[11px] font-semibold text-foreground"
			text-anchor="middle"
		>
			{formatDistanceM(sample?.distanceM, units)}
		</text>
	</svg>
</div>
