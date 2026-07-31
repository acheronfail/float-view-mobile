<script lang="ts">
  import type { LiveSample } from '../../backend/contract';
  import GaugeScale from './GaugeScale.svelte';

  let {
    sample,
    batterySeriesCells,
  }: {
    sample: LiveSample | null | undefined;
    batterySeriesCells: number | null | undefined;
  } = $props();

  const green = [22, 163, 74];
  const yellow = [234, 179, 8];
  const red = [220, 38, 38];
  const tickValues = Array.from({ length: 11 }, (_, index) => index * 10);
  const labelValues = Array.from({ length: 6 }, (_, index) => index * 20);

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
</script>

<section class="rounded-2xl border border-outline bg-surface p-3 sm:p-5" aria-labelledby="power-title">
  <div class="mb-3 flex items-baseline justify-between gap-3">
    <h2 id="power-title" class="text-xs font-semibold tracking-[0.16em] text-muted uppercase">
      Battery
      {#if charging}
        <span class="mt-2 text-right text-[0.625rem] font-semibold tracking-[0.14em] text-warning uppercase">
          Charging
        </span>
      {/if}
    </h2>
    <span
      class:text-muted={sample?.packVoltageV == null}
      class:text-foreground={sample?.packVoltageV != null}
      class="font-mono text-sm font-semibold tabular-nums"
    >
      {#if !sample?.packVoltageV}
        —
      {:else}
        <span class="text-xs text-muted uppercase">Pack</span>
        <span>{sample.packVoltageV.toFixed(1)} V</span>
        {#if batterySeriesCells}
          <span class="text-xs text-muted uppercase">Cell</span>
          <span>{(sample.packVoltageV / batterySeriesCells).toFixed(2)} V</span>
        {/if}
      {/if}
    </span>
  </div>

  <div class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3">
    <div
      class="relative h-5 min-w-0 flex-1 overflow-hidden rounded-full bg-elevated"
      role="progressbar"
      aria-label="Battery level"
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={batteryPercent ?? undefined}
      aria-valuetext={batteryPercent == null ? 'Unavailable' : `${batteryPercent.toFixed(0)} percent`}
    >
      <GaugeScale values={tickValues} maximum={100} />
      <div
        class={`relative h-full rounded-full transition-[width,background-color] duration-500 ${
          charging
            ? 'animate-[battery-charging-pulse_1s_ease-in-out_infinite_alternate] bg-linear-to-r from-orange-600 via-amber-500 to-orange-600 bg-[length:200%_100%] motion-reduce:animate-none'
            : 'bg-[var(--battery-color)]'
        }`}
        style={`width: ${batteryWidth}%; --battery-color: ${batteryColor}`}
      ></div>
    </div>

    <output
      class:text-muted={batteryPercent == null}
      class="min-w-[2.5rem] justify-self-end text-right font-mono text-sm font-semibold tabular-nums"
    >
      {batteryPercent == null ? '—' : `${batteryPercent.toFixed(0)}%`}
    </output>

    <div class="mt-1 min-w-0">
      <GaugeScale values={labelValues} maximum={100} labels format={(value) => `${value}%`} />
    </div>
  </div>
</section>

<style>
  @keyframes -global-battery-charging-pulse {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: 0 0;
    }
  }
</style>
