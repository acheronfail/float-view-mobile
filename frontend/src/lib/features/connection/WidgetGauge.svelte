<script lang="ts">
  import LiveValueBox from './LiveValueBox.svelte';
  import MarkerCapture from './MarkerCapture.svelte';
  import { prefersReducedMotion, Tween } from 'svelte/motion';
  import type { ControllerCurrentLimits, LiveSample, UnitPreference } from '../../backend/contract';
  import { formatSpeedKph } from '../measurements/format';
  import { driveLoad } from './gauge';

  let {
    sessionId = null,
    sample,
    currentLimits,
    units,
  }: {
    sessionId?: string | null;
    sample: LiveSample | null | undefined;
    currentLimits: ControllerCurrentLimits | null | undefined;
    units: UnitPreference;
  } = $props();

  const center = 160;
  const outerRadius = 124;
  const innerRadius = 112;
  const startAngle = 135;
  const endAngle = 405;

  const speedMaximum = $derived(units === 'metric' ? 60 : 40);
  const speedUnit = $derived(units === 'metric' ? 'km/h' : 'mph');
  const speed = $derived(
    sample?.speedKph == null ? null : Math.abs(sample.speedKph) * (units === 'metric' ? 1 : 0.621371)
  );
  const load = $derived(driveLoad(sample, currentLimits));
  const animatedSpeed = new Tween(0);
  const animatedLoad = new Tween(0);
  $effect(() => {
    void animatedSpeed.set(speed ?? 0, {
      duration: prefersReducedMotion.current ? 0 : 150,
    });
  });

  $effect(() => {
    void animatedLoad.set(load?.percentage ?? 0, {
      duration: prefersReducedMotion.current ? 0 : 150,
    });
  });

  const loadLabel = $derived(
    load?.source === 'duty'
      ? 'Duty'
      : load?.source === 'motor'
        ? 'Motor'
        : load?.source === 'battery'
          ? 'Battery'
          : 'Drive Load'
  );
  const loadStrokeClass = $derived(
    load?.source === 'duty'
      ? 'stroke-duty'
      : load?.source === 'motor'
        ? 'stroke-motor'
        : load?.source === 'battery'
          ? 'stroke-battery-current'
          : 'stroke-muted'
  );
  const loadTextClass = $derived(
    load?.source === 'duty'
      ? 'text-duty'
      : load?.source === 'motor'
        ? 'text-motor'
        : load?.source === 'battery'
          ? 'text-battery-current'
          : 'text-muted'
  );

  function point(radius: number, angle: number) {
    const radians = (angle * Math.PI) / 180;
    return { x: center + radius * Math.cos(radians), y: center + radius * Math.sin(radians) };
  }

  function arc(radius: number) {
    const start = point(radius, startAngle);
    const end = point(radius, endAngle);
    return `M ${start.x} ${start.y} A ${radius} ${radius} 0 1 1 ${end.x} ${end.y}`;
  }

  function ticks(maximum: number, step: number, outerTickRadius: number, innerTickRadius: number, labelRadius: number) {
    return Array.from({ length: maximum / step + 1 }, (_, index) => {
      const value = index * step;
      const angle = startAngle + (value / maximum) * (endAngle - startAngle);
      return {
        value,
        outer: point(outerTickRadius, angle),
        inner: point(innerTickRadius, angle),
        label: point(labelRadius, angle),
      };
    });
  }

  const speedTicks = $derived(ticks(speedMaximum, 5, outerRadius + 9, outerRadius + 17, outerRadius + 29));
  const dutyTicks = ticks(100, 10, innerRadius - 6, innerRadius - 14, innerRadius - 24);

  function progress(value: number | null, maximum: number) {
    return value == null ? 0 : (Math.min(Math.max(value, 0), maximum) / maximum) * 100;
  }
</script>

<div class="relative flex h-full min-h-0 items-center justify-center overflow-hidden">
  <svg
    viewBox="0 0 320 300"
    class="mx-auto block h-full max-h-full w-auto max-w-full overflow-visible text-foreground"
    role="group"
    aria-labelledby="gauge-title gauge-description"
  >
    <title id="gauge-title">Live speed and drive load gauge</title>
    <desc id="gauge-description">
      Speed is {formatSpeedKph(sample?.speedKph, units)} and {loadLabel.toLowerCase()} is {load == null
        ? 'unavailable'
        : `${load.percentage.toFixed(0)} percent`}.
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
      stroke-dasharray={`${progress(speed == null ? null : animatedSpeed.current, speedMaximum)} 100`}
    />
    <path
      d={arc(innerRadius)}
      pathLength="100"
      class="fill-none stroke-outline [stroke-width:9] opacity-55 [stroke-linecap:round]"
    />
    <path
      d={arc(innerRadius)}
      pathLength="100"
      class={`fill-none ${loadStrokeClass} [stroke-width:9] transition-colors [stroke-linecap:round]`}
      stroke-dasharray={`${progress(load == null ? null : animatedLoad.current, 100)} 100`}
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
      {speed == null ? '—' : animatedSpeed.current.toFixed(1)}
    </text>
    <text
      x="160"
      y="163"
      class={`fill-current text-[9px] font-semibold tracking-[0.12em] ${loadTextClass} transition-colors`}
      text-anchor="middle"
    >
      {loadLabel}
    </text>
    <text
      x="160"
      y="188"
      class={`fill-current font-mono text-[25px] font-bold tabular-nums ${loadTextClass} transition-colors`}
      text-anchor="middle"
    >
      {load == null ? '—' : `${animatedLoad.current.toFixed(0)}%`}
    </text>
    <foreignObject class="overflow-visible" x="105" y="232" width="110" height="56">
      <LiveValueBox {sample} slot={4} />
    </foreignObject>
  </svg>
  <MarkerCapture {sessionId} />
</div>
