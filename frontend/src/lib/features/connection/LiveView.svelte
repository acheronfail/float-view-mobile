<script lang="ts">
  let {
    onconfigure,
    onbms,
    onenablerefloat,
    nowMs,
    charging = false,
  }: {
    onconfigure?: (trigger: HTMLButtonElement) => void;
    onbms?: (trigger: HTMLButtonElement) => void;
    onenablerefloat: () => void;
    nowMs: number;
    charging?: boolean;
  } = $props();
  import { backendState } from '$lib/backend/state.svelte';
  import { readingsAreDimmed, refloatMenuActions } from './live';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import Icon from '$lib/ui/Icon.svelte';
  import WidgetNumbers from './WidgetNumbers.svelte';
  import WidgetGauge from './WidgetGauge.svelte';
  import WidgetPower from './WidgetPower.svelte';
  import WidgetTemperatures from './WidgetTemperatures.svelte';
  import WidgetFootpads from './WidgetFootpads.svelte';

  const connection = $derived(backendState.snapshot?.activeConnection);
  const sample = $derived(connection?.latestSample);

  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const dimmed = $derived(connection ? readingsAreDimmed(connection) : false);
  const enableRefloat = $derived(
    connection
      ? refloatMenuActions(connection).find(
          (action) => action.kind === 'control' && action.control === 'package' && action.enabled
        )
      : undefined
  );

  function enable() {
    if (enableRefloat && !enableRefloat.disabled && !backendState.busy) onenablerefloat();
  }
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
        {#if connection.refloatControls?.packageEnabled === false}
          <div
            class="absolute inset-x-2 top-2 flex items-center justify-between gap-2 rounded-xl border border-danger bg-surface px-3 py-2 text-danger shadow-lg"
          >
            <p role="status" class="flex items-center gap-2 text-sm font-semibold">
              <Icon name="triangle-alert" class="shrink-0 text-xl" />
              Refloat is disabled
            </p>
            <button
              type="button"
              class="min-h-11 shrink-0 touch-manipulation rounded-lg border border-danger bg-danger/10 px-3 py-2 text-sm font-semibold disabled:opacity-50"
              disabled={!enableRefloat || enableRefloat.disabled || backendState.busy}
              aria-label="Enable Refloat"
              aria-busy={enableRefloat?.pending || undefined}
              onpointerdown={(event) => activateFromPointer(event, enable, { scrollable: true })}
              onclick={(event) => activateFromKeyboardOrAssistiveClick(event, enable)}
              >{enableRefloat?.pending ? 'Enabling…' : 'Enable'}</button
            >
          </div>
        {/if}
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
