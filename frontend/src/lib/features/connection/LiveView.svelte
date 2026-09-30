<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { readingsAreDimmed } from './live';
  import WidgetGauge from './WidgetGauge.svelte';
  import WidgetPower from './WidgetPower.svelte';
  import WidgetTemperatures from './WidgetTemperatures.svelte';
  import WidgetFootpads from './WidgetFootpads.svelte';

  const connection = $derived(backendState.snapshot?.activeConnection);
  const sample = $derived(connection?.latestSample);
  const currentLimits = $derived(connection?.currentLimits);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const dimmed = $derived(connection ? readingsAreDimmed(connection) : false);
</script>

{#if connection}
  <section aria-label="Live board widgets" class="flex h-full min-h-0 flex-col gap-2">
    {#if connection.trackingMode === 'guest'}
      <p
        class="flex shrink-0 items-center justify-between gap-3 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-sm"
        role="status"
      >
        <span>Guest connection — live data will not be saved or tracked.</span>
      </p>
    {/if}
    <div class="grid min-h-0 flex-1 grid-rows-[minmax(4rem,auto)_minmax(0,1fr)_auto_auto] gap-2">
      <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
        <WidgetFootpads {sample} />
      </div>

      <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
        <WidgetGauge
          sessionId={connection.trackingMode === 'saved' ? (connection.activeSession?.sessionId ?? null) : null}
          {sample}
          {currentLimits}
          {units}
        />
      </div>

      <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
        <WidgetTemperatures {sample} />
      </div>

      <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
        <WidgetPower {sample} estimate={connection.batteryEstimate} />
      </div>
    </div>
  </section>
{:else}
  <p class="text-muted">Recovering active connection…</p>
{/if}
