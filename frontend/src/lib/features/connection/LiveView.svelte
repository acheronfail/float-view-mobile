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
  <section
    aria-label="Live board widgets"
    class="grid h-full min-h-0 grid-rows-[minmax(4rem,auto)_minmax(0,1fr)_auto_auto] gap-2"
  >

    <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
      <WidgetFootpads {sample} />
    </div>

    <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
      <WidgetGauge {sample} {currentLimits} {units} />
    </div>

    <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
      <WidgetTemperatures {sample} />
    </div>

    <div class:opacity-45={dimmed} class="min-h-0 overflow-hidden transition-opacity">
      <WidgetPower {sample} batterySeriesCells={currentLimits?.batterySeriesCells} />
    </div>
  </section>
{:else}
  <p class="text-muted">Recovering active connection…</p>
{/if}
