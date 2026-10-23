<script lang="ts">
  import { backDismissal } from '$lib/ui/back-dismissal';
  import '$lib/ui/telemetry-chart.css';
  import { createMotion } from '$lib/ui/motion';
  import { onMount } from 'svelte';
  import LiveChartPlot from './LiveChartPlot.svelte';
  import type { LiveHistory } from './live-history';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import { bmsMenuVisible } from './bms';
  import { liveChartColor, liveChartGroups, liveChartReading } from './live-charts';

  const motion = createMotion();
  let { onclose, history }: { onclose: (keyboard: boolean) => void; history: LiveHistory } = $props();
  let dialog: HTMLDialogElement;
  const connection = $derived(backendState.snapshot?.activeConnection);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const chartGroups = $derived(liveChartGroups(bmsMenuVisible(connection?.bms)));
  const stale = $derived(history.stale);
  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog
  use:backDismissal={{ close: () => onclose(true) }}
  use:motion.surface={'panel'}
  bind:this={dialog}
  aria-labelledby="live-charts-title"
  class="fixed inset-0 m-0 h-dvh max-h-none w-screen max-w-none bg-canvas p-0 text-foreground backdrop:bg-canvas"
  oncancel={(event) => {
    event.preventDefault();
    event.stopPropagation();
    onclose(true);
  }}
>
  <div
    class="app-safe-area app-safe-area-without-tabs flex h-full min-h-0 flex-col"
    style="padding-left: 0; padding-right: 0"
  >
    <header class="flex shrink-0 items-center justify-between gap-3 border-b border-outline px-4 pb-3">
      <div>
        <h2 id="live-charts-title" class="text-xl font-bold">Live charts</h2>
        <p class="mt-1 text-sm text-muted">{connection?.displayName ?? 'Board disconnected'} · Last 30 seconds</p>
      </div>
      <button
        type="button"
        class="min-h-11 touch-manipulation rounded-lg border border-outline px-4"
        onpointerdown={(event) => activateFromPointer(event, () => onclose(false))}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => onclose(true))}>Done</button
      >
    </header>
    <div class="flex shrink-0 items-center justify-between gap-3 px-4 py-3 text-xs text-muted">
      <span class="flex items-center gap-2" role="status"
        ><span class="h-2 w-2 rounded-full" class:bg-success={!stale} class:bg-warning={stale}></span>{stale
          ? 'Waiting for live data'
          : 'Live'}</span
      >
      <span>History is available in Session</span>
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain">
      {#each chartGroups as group (group.title)}
        <section
          aria-label={group.title}
          class="nonselectable-surface mb-4 border-t border-outline bg-surface pt-3 pb-2"
        >
          <h3 class="mb-2 px-4 text-xs font-semibold tracking-wider text-muted uppercase">{group.title}</h3>
          <div class="mb-3 flex flex-wrap gap-x-5 gap-y-1 px-4">
            {#each group.fields as field, index (field)}
              {@const reading = liveChartReading(
                field,
                stale ? null : connection?.latestSample,
                units,
                stale ? null : connection?.bms,
                Date.now()
              )}
              <div class="flex items-baseline gap-2 text-sm">
                <span
                  class="h-2 w-2 self-center rounded-full"
                  style:background={liveChartColor(group.fields[index], index)}
                ></span>
                <span class="text-muted">{reading.label}</span>
                <span class="font-mono font-semibold tabular-nums">{reading.value?.toFixed(reading.digits) ?? '—'}</span
                >
                <span class="text-xs text-muted">{reading.unit}</span>
              </div>
            {/each}
          </div>
          <LiveChartPlot {history} fields={group.fields} {units} title={group.title} />
        </section>
      {/each}
    </div>
  </div>
</dialog>
