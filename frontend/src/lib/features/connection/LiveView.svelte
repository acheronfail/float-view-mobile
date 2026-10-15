<script lang="ts">
  let {
    onconfigure,
    onbms,
    nowMs,
    charging = false,
  }: {
    onconfigure?: (trigger: HTMLButtonElement) => void;
    onbms?: () => void;
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
    {#if connection.trackingMode === 'guest'}
      <p
        class="flex shrink-0 items-center justify-between gap-3 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-sm"
        role="status"
      >
        <span>Guest connection — live data will not be saved or tracked.</span>
      </p>
    {/if}
    <div class="grid min-h-0 flex-1 grid-rows-[2.5rem_auto_minmax(14rem,1fr)_auto_auto] gap-2">
      <div
        class:opacity-45={dimmed}
        class="motion-feedback min-h-0 overflow-hidden transition-opacity motion-reduce:transition-none"
      >
        <WidgetFootpads {sample} />
      </div>

      <div class:opacity-45={dimmed} class="motion-feedback min-h-0 transition-opacity motion-reduce:transition-none">
        <WidgetNumbers {sample} />
      </div>

      <div
        class:opacity-45={dimmed}
        class="motion-feedback min-h-0 overflow-hidden transition-opacity motion-reduce:transition-none"
      >
        <WidgetGauge
          sessionId={connection.trackingMode === 'saved' ? (connection.activeSession?.sessionId ?? null) : null}
          {sample}
          {units}
        />
      </div>

      <div class:opacity-45={dimmed} class="motion-feedback transition-opacity motion-reduce:transition-none">
        <WidgetTemperatures {sample} bms={connection.bms} {nowMs} />
      </div>

      <div
        class:opacity-45={dimmed}
        class="motion-feedback min-h-0 overflow-hidden transition-opacity motion-reduce:transition-none"
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
