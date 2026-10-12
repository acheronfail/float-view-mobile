<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import { liveValue } from './live-values';
  import { liveChartGeometry, liveChartGroups, retainLivePoints, type LiveChartPoint } from './live-charts';
  let { onclose }: { onclose: (keyboard: boolean) => void } = $props();
  let dialog: HTMLDialogElement;
  let now = $state(performance.now());
  let points = $state<LiveChartPoint[]>([]);
  let lastSequence: number | null = null;
  let lastDevice: string | null = null;
  const colors = ['var(--color-accent)', '#f59e0b', '#a78bfa'];
  const connection = $derived(backendState.snapshot?.activeConnection);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const stale = $derived(
    connection?.state !== 'ready' || !points.length || now - points[points.length - 1].time > 1_500
  );
  $effect(() => {
    const sample = connection?.latestSample;
    const device = connection?.deviceId ?? null;
    const ready = connection?.state === 'ready';
    untrack(() => {
      if (device !== lastDevice || (sample && lastSequence !== null && sample.sequence < lastSequence)) {
        points = [];
        lastSequence = null;
        lastDevice = device;
      }
      if (!ready || !sample || sample.sequence === lastSequence || Date.now() - Date.parse(sample.observedAt) > 1_500)
        return;
      lastSequence = sample.sequence;
      now = performance.now();
      points = retainLivePoints([...points, { time: now, sample }], now);
    });
  });
  onMount(() => {
    dialog.showModal();
    const timer = setInterval(() => {
      now = performance.now();
      points = retainLivePoints(points, now);
    }, 100);
    return () => clearInterval(timer);
  });
</script>

<dialog
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
      {#each liveChartGroups as group (group.title)}
        {@const geometry = liveChartGeometry(points, group.fields, units, now)}
        <section aria-label={group.title} class="mb-4 border-t border-outline bg-surface pt-3 pb-2">
          <h3 class="mb-2 px-4 text-xs font-semibold tracking-wider text-muted uppercase">{group.title}</h3>
          <div class="mb-3 flex flex-wrap gap-x-5 gap-y-1 px-4">
            {#each group.fields as field, index (field)}
              {@const reading = liveValue(field, stale ? null : connection?.latestSample, units)}
              <div class="flex items-baseline gap-2 text-sm">
                <span class="h-2 w-2 self-center rounded-full" style:background={colors[index]}></span>
                <span class="text-muted">{reading.label}</span>
                <span class="font-mono font-semibold tabular-nums">{reading.value?.toFixed(reading.digits) ?? '—'}</span
                >
                <span class="text-xs text-muted">{reading.unit}</span>
              </div>
            {/each}
          </div>
          <div class="relative">
            <div
              class="pointer-events-none absolute inset-y-0 left-3 z-10 flex flex-col items-start justify-between font-mono text-[10px] text-muted"
            >
              <span class="bg-surface/80 px-1">{geometry.high.toFixed(0)}</span><span class="bg-surface/80 px-1"
                >{((geometry.high + geometry.low) / 2).toFixed(0)}</span
              ><span class="bg-surface/80 px-1">{geometry.low.toFixed(0)}</span>
            </div>
            <svg
              class="block h-24 w-full overflow-hidden"
              viewBox="0 0 1000 100"
              preserveAspectRatio="none"
              role="img"
              aria-label={`${group.title}, rolling 30-second chart`}
            >
              {#each [0, 50, 100] as y (y)}<line
                  x1="0"
                  x2="1000"
                  y1={y}
                  y2={y}
                  stroke="var(--color-outline)"
                  vector-effect="non-scaling-stroke"
                />{/each}
              {#each [0, 250, 500, 750, 1000] as x (x)}<line
                  x1={x}
                  x2={x}
                  y1="0"
                  y2="100"
                  stroke="var(--color-outline)"
                  stroke-dasharray="2 5"
                  vector-effect="non-scaling-stroke"
                />{/each}
              {#each geometry.paths as path, index (group.fields[index])}<path
                  d={path}
                  fill="none"
                  stroke={colors[index]}
                  stroke-width="2"
                  stroke-linejoin="round"
                  vector-effect="non-scaling-stroke"
                />{/each}
            </svg>
          </div>
          <div class="mt-1 flex justify-between px-4 font-mono text-[10px] text-muted">
            <span>−30s</span><span>−15s</span><span>Now</span>
          </div>
        </section>
      {/each}
    </div>
  </div>
</dialog>
