<script lang="ts">
  import { onMount } from 'svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import type { LiveSample, BatteryEstimate } from '../../backend/contract';
  import { colorForBattery } from './gauge-colors';
  import { batteryPresentation } from './battery';
  import BatterySetupWarning from './BatterySetupWarning.svelte';
  import GaugeBar from './WidgetBar.svelte';

  let {
    sample,
    idPrefix = 'power',
    estimate,
    charging = sample?.charging === true,
    savedBoard = false,
    dashboard = false,
    expanded = false,
    onconfigure,
    onbms,
  }: {
    idPrefix?: string;
    charging?: boolean;
    savedBoard?: boolean;
    dashboard?: boolean;
    expanded?: boolean;
    onbms?: (trigger: HTMLButtonElement) => void;
    onconfigure?: (trigger: HTMLButtonElement) => void;
    sample: LiveSample | null | undefined;
    estimate: BatteryEstimate | null | undefined;
  } = $props();

  let bmsButton = $state<HTMLButtonElement>();
  let activationFrame: number | undefined;
  onMount(() => () => {
    if (activationFrame !== undefined) cancelAnimationFrame(activationFrame);
  });

  function openBmsFromPointer() {
    if (activationFrame !== undefined) return;
    // Let synthesized touch mouse events finish before the overlay takes focus.
    activationFrame = requestAnimationFrame(() => {
      activationFrame = undefined;
      if (bmsButton) onbms?.(bmsButton);
    });
  }

  const labelValues = Array.from({ length: 6 }, (_, index) => index * 20);

  const battery = $derived(batteryPresentation(estimate, sample?.batteryLevel, savedBoard));
  const batteryPercent = $derived(battery.percent);
  const batterySeriesCells = $derived(estimate?.seriesCount);
  const batteryColor = $derived(colorForBattery(batteryPercent));
</script>

{#if dashboard}
  <section class="dashboard-power h-full min-h-0 w-full" aria-labelledby={`${idPrefix}-title`}>
    <div class="power-content" class:expanded>
      <div class="power-labels">
        <h2 id={`${idPrefix}-title`} class="live-widget-title" class:text-warning={charging}>
          {charging ? 'Charging' : 'Battery'}
        </h2>
        <span class="pack-voltage"
          >Pack {sample?.packVoltageV == null ? '—' : `${sample.packVoltageV.toFixed(1)} V`}</span
        >
      </div>
      <p class="power-value font-mono font-semibold tabular-nums">
        {batteryPercent == null ? '—' : `${batteryPercent.toFixed(0)}%`}
      </p>
      <div class="power-bar">
        <GaugeBar
          value={batteryPercent}
          maximum={100}
          ariaLabel={battery.label}
          ariaValueText={batteryPercent == null ? 'Unavailable' : `${batteryPercent.toFixed(0)} percent`}
          showLabels={false}
          showTicks={false}
          heightClass="h-full min-h-[6px]"
          fillClass={charging ? 'bg-warning' : 'bg-[var(--battery-color)]'}
          fillStyle={`--battery-color: ${batteryColor}`}
        />
      </div>
      <div class="power-scale text-muted" aria-hidden="true"><span>0%</span><span>50%</span><span>100%</span></div>
    </div>
  </section>
{:else}
  <section class="relative flex h-full min-h-0 flex-col overflow-hidden" aria-labelledby={`${idPrefix}-title`}>
    {#if onbms}
      <button
        type="button"
        class="absolute inset-0 z-10 touch-manipulation rounded-sm"
        bind:this={bmsButton}
        aria-label="Open battery management system"
        onpointerdown={(event) => activateFromPointer(event, openBmsFromPointer, { scrollable: true })}
        onclick={(event) =>
          activateFromKeyboardOrAssistiveClick(event, () => {
            if (bmsButton) onbms?.(bmsButton);
          })}
      ></button>
    {/if}
    <div class="power-heading mb-3 flex shrink-0 items-baseline justify-between gap-3">
      <h2
        id={`${idPrefix}-title`}
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
      {#if battery.needsSetup && onconfigure}
        <div class="relative z-20"><BatterySetupWarning {onconfigure} /></div>
      {/if}
    </div>
  </section>
{/if}

<style>
  .dashboard-power {
    container: dashboard-power / size;
  }
  .power-content {
    display: flex;
    height: 100%;
    min-height: 0;
    flex-direction: column;
    justify-content: center;
  }
  .power-labels {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0 0.5rem;
    font-size: 10px;
    line-height: 12px;
  }
  .power-labels h2 {
    font-weight: 600;
    text-transform: uppercase;
    color: var(--color-muted);
  }
  .pack-voltage,
  .power-scale {
    display: none;
  }
  .power-value {
    font-size: 14px;
    line-height: 20px;
  }
  .power-bar {
    height: 6px;
    flex-shrink: 0;
  }
  .power-bar :global(> div),
  .power-bar :global(> div > div) {
    height: 100%;
  }
  @container dashboard-power (min-height: 65px) {
    .power-value {
      font-size: 20px;
      line-height: 28px;
    }
    .power-bar {
      height: 10px;
    }
  }
  @container dashboard-power (min-height: 90px) {
    .pack-voltage {
      display: inline;
      white-space: nowrap;
    }
  }
  @container dashboard-power (min-width: 160px) and (min-height: 65px) {
    .power-scale {
      display: flex;
      justify-content: space-between;
      margin-top: 2px;
      font-size: 9px;
      line-height: 11px;
    }
  }
  @container dashboard-power (min-width: 240px) and (min-height: 90px) {
    .power-bar {
      height: 16px;
    }
  }

  @container dashboard-power (min-width: 120px) and (min-height: 65px) {
    .power-content.expanded {
      justify-content: space-evenly;
    }
    .expanded .power-value {
      font-size: clamp(20px, min(32cqh, 20cqi), 56px);
      line-height: 1.15;
    }
    .expanded .power-bar {
      height: clamp(10px, 15cqh, 24px);
    }
  }

  @media (max-height: 40rem) {
    .power-heading {
      margin-bottom: 0.25rem;
    }
  }

  @keyframes -global-battery-charging-pulse {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: 0 0;
    }
  }
</style>
