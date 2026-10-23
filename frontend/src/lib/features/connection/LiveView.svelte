<script lang="ts">
  let {
    onconfigure,
    onbms,
    nowMs,
    charging = false,
  }: {
    onconfigure?: (trigger: HTMLButtonElement) => void;
    onbms?: (trigger: HTMLButtonElement) => void;
    nowMs: number;
    charging?: boolean;
  } = $props();
  import { backendState } from '$lib/backend/state.svelte';
  import { readingsAreDimmed } from './live';
  import WidgetNumbers from './WidgetNumbers.svelte';
  import WidgetGauge from './WidgetGauge.svelte';
  import WidgetPower from './WidgetPower.svelte';
  import WidgetTemperatures from './WidgetTemperatures.svelte';
  import WidgetFootpads from './WidgetFootpads.svelte';

  const connection = $derived(backendState.snapshot?.activeConnection);
  const sample = $derived(connection?.latestSample);

  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const dimmed = $derived(connection ? readingsAreDimmed(connection) : false);
</script>

{#if connection}
  <section
    aria-label="Live board widgets"
    class="nonselectable-surface flex h-full min-h-0 flex-col gap-2 overflow-y-auto"
  >
    <div class="live-widgets min-h-0 flex-1">
      <div
        class:opacity-45={dimmed}
        class="live-footpads motion-feedback min-h-0 overflow-hidden transition-opacity motion-reduce:transition-none"
      >
        <WidgetFootpads {sample} />
      </div>

      <div
        class:opacity-45={dimmed}
        class="live-numbers motion-feedback min-h-0 transition-opacity motion-reduce:transition-none"
      >
        <WidgetNumbers {sample} />
      </div>

      <div
        class:opacity-45={dimmed}
        class="live-gauge motion-feedback relative min-h-0 overflow-hidden transition-opacity motion-reduce:transition-none"
      >
        <WidgetGauge
          sessionId={connection.trackingMode === 'saved' ? (connection.activeSession?.sessionId ?? null) : null}
          {sample}
          {units}
        />
      </div>

      <div
        class:opacity-45={dimmed}
        class="live-temperatures motion-feedback transition-opacity motion-reduce:transition-none"
      >
        <WidgetTemperatures {sample} bms={connection.bms} {nowMs} />
      </div>

      <div
        class:opacity-45={dimmed}
        class="live-power motion-feedback transition-opacity motion-reduce:transition-none"
      >
        <WidgetPower
          {charging}
          {onbms}
          {onconfigure}
          savedBoard={!!connection?.savedDeviceId && connection?.deviceKind === 'board'}
          {sample}
          estimate={connection.batteryEstimate}
        />
      </div>
    </div>
  </section>
{:else}
  <p class="text-muted">Recovering active connection…</p>
{/if}

<style>
  .live-widgets {
    display: grid;
    grid-template-areas: 'footpads' 'numbers' 'gauge' 'temperatures' 'power';
    grid-template-rows: 2.5rem 3.5rem minmax(11rem, 1fr) max-content max-content;
    gap: 0.5rem;
  }
  .live-footpads {
    grid-area: footpads;
  }
  .live-numbers {
    grid-area: numbers;
  }
  .live-gauge {
    grid-area: gauge;
  }
  .live-temperatures {
    grid-area: temperatures;
  }
  .live-power {
    grid-area: power;
  }

  @media (max-height: 40rem) {
    .live-widgets {
      grid-template-rows: 2rem 3.5rem minmax(11rem, 1fr) max-content max-content;
      gap: 0.375rem;
    }
  }

  @media (min-width: 30rem) and (max-height: 32rem) and (orientation: landscape) {
    .live-widgets {
      grid-template-areas: 'gauge footpads' 'gauge numbers' 'gauge temperatures' 'gauge power';
      grid-template-columns: minmax(12rem, 0.9fr) minmax(0, 1fr);
      grid-template-rows: 2rem 3.5rem max-content max-content;
      column-gap: 1rem;
    }
  }
</style>
