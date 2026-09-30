<script lang="ts">
  import type { LiveSample, BatteryEstimate } from '../../backend/contract';
  import { batteryPresentation } from './battery';
  import BatterySetupWarning from './BatterySetupWarning.svelte';
  import GaugeBar from './WidgetBar.svelte';

  let {
    sample,
    estimate,
    savedBoard = false,
    onconfigure,
  }: {
    savedBoard?: boolean;
    onconfigure?: (trigger: HTMLButtonElement) => void;
    sample: LiveSample | null | undefined;
    estimate: BatteryEstimate | null | undefined;
  } = $props();

  const green = [22, 163, 74];
  const yellow = [234, 179, 8];
  const red = [220, 38, 38];
  const labelValues = Array.from({ length: 6 }, (_, index) => index * 20);

  const battery = $derived(batteryPresentation(estimate, sample?.batteryLevel, savedBoard));
  const batteryPercent = $derived(battery.percent);
  const batterySeriesCells = $derived(estimate?.seriesCount);
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

<section class="flex h-full min-h-0 flex-col overflow-hidden" aria-labelledby="power-title">
  <div class="mb-3 flex shrink-0 items-baseline justify-between gap-3">
    <h2
      id="power-title"
      class="text-xs font-semibold tracking-[0.16em] text-muted uppercase tall:text-sm taller:text-base"
    >
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
        <span class="text-xs text-muted uppercase tall:text-xs taller:text-sm">Pack</span>
        <span>{sample.packVoltageV.toFixed(1)} V</span>
        {#if batterySeriesCells}
          <span class="text-xs text-muted uppercase tall:text-xs taller:text-sm">Cell</span>
          <span style:color={batteryPercent == null ? undefined : batteryColor}>
            {(sample.packVoltageV / batterySeriesCells).toFixed(2)} V
          </span>
        {/if}
      {/if}
    </span>
  </div>

  <div class="flex min-h-0 flex-1 items-start gap-1">
    <div class="min-w-0 flex-1">
      <GaugeBar
        value={batteryPercent}
        maximum={100}
        detail={batteryPercent == null ? '—' : `${batteryPercent.toFixed(0)}%`}
        detailClass={batteryPercent == null ? 'text-muted' : ''}
        ariaLabel={battery.label}
        ariaValueText={batteryPercent == null ? 'Unavailable' : `${batteryPercent.toFixed(0)} percent`}
        showLabels
        tickValues={labelValues}
        format={(value) => `${value}%`}
        heightClass="h-5"
        fillClass={charging
          ? 'animate-[battery-charging-pulse_1s_ease-in-out_infinite_alternate] bg-linear-to-r from-orange-600 via-amber-500 to-orange-600 bg-[length:200%_100%] motion-reduce:animate-none'
          : 'bg-[var(--battery-color)]'}
        fillStyle={`--battery-color: ${batteryColor}`}
      />
    </div>
    {#if battery.needsSetup && onconfigure}<BatterySetupWarning {onconfigure} />{/if}
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
