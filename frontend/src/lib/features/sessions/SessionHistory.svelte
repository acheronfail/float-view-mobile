<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import type { SessionMetadata } from '$lib/backend/contract';
  import type { SessionReviewController } from '$lib/backend/session-controller';
  import type { SessionSeriesResult } from '$lib/backend/session-data';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { axisTicks, buildPathSegments, finiteDomain, formatAxisTick, zeroReference } from './history-geometry';
  import { clampViewport, panViewport, pinchViewport, type Viewport } from './viewport';

  let { metadata }: { metadata: SessionMetadata } = $props();
  let plot = $state<HTMLElement | null>(null);
  let width = $state(0);
  let viewport = $state<Viewport>({ startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs });
  let previewViewport = $state<Viewport | null>(null);
  let result = $state<SessionSeriesResult | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let groupResults = new SvelteMap<number, SessionSeriesResult>();
  let groupLoading = new SvelteMap<number, boolean>();
  let groupErrors = new SvelteMap<number, string>();
  // Keep the in-flight guard outside Svelte's reactive graph. The initial
  // loading effect must not rerun just because it updates the loading map.
  const inFlightGroups = new Set<number>();
  let groupNodes = new SvelteMap<number, HTMLElement>();
  let groupObserver: IntersectionObserver | null = null;
  let resizeFrame: number | null = null;
  let reviewController: SessionReviewController | null = null;
  let activeField = $state('');
  let inspectionMs = $state(0);
  let exactValue = $state<number | null>(null);
  let exactStatus = $state<'idle' | 'pending' | 'exact' | 'unavailable' | 'error'>('idle');
  let inspectionFrame: number | null = null;
  let inspectionDebounce: number | null = null;
  let inspectionInFlight = false;
  let inspectionRequestVersion = 0;
  let queuedInspection: number | null = null;

  const INSPECTION_DEBOUNCE_MS = 60;
  const CHART_WIDTH = 320;
  const CHART_HEIGHT = 96;
  const Y_TICK_COUNT = 5;
  const MIN_SELECTION_PIXELS = 8;

  const fields = $derived(metadata.fields.slice(0, 16));
  const fieldGroups = $derived.by(() => {
    const groups: SessionMetadata['fields'][] = [];
    let stable: SessionMetadata['fields'] = [];
    fields.forEach((field) => {
      if (field.source === 'stable' && stable.length < 9) {
        stable = [...stable, field];
        return;
      }
      if (!groups.length || groups.at(-1)!.length >= 8) groups.push([]);
      groups[groups.length - 1] = [...groups.at(-1)!, field];
    });
    if (stable.length) groups.unshift(stable);
    return groups;
  });
  const visibleViewport = $derived(previewViewport ?? viewport);
  const span = $derived(Math.max(1, visibleViewport.endElapsedMs - visibleViewport.startElapsedMs));
  const duration = $derived(Math.max(1, metadata.summary.totalElapsedMs));
  let pointers = new SvelteMap<number, { x: number; y: number }>();
  let gestureStart: { viewport: Viewport; distance: number; ratio: number; pixelWidth: number } | null = null;
  let selectionStartElapsedMs = $state<number | null>(null);
  let selectionEndElapsedMs = $state<number | null>(null);
  const selectionRange = $derived.by(() => {
    if (selectionStartElapsedMs === null || selectionEndElapsedMs === null) return null;
    const startX = ((selectionStartElapsedMs - visibleViewport.startElapsedMs) / span) * CHART_WIDTH;
    const endX = ((selectionEndElapsedMs - visibleViewport.startElapsedMs) / span) * CHART_WIDTH;
    return {
      startX,
      endX,
      leftX: Math.min(startX, endX),
      rightX: Math.max(startX, endX),
    };
  });
  let inspectionPinned = $state(false);
  let inspectionVisible = $derived(exactStatus !== 'idle' || inspectionPinned);

  $effect(() => {
    if (!activeField && fields.length) activeField = fields[0]!.name;
    if (!width || !fieldGroups.length) return;
    void loadSeriesGroup(0, width);
  });

  function rebuildResult() {
    const loaded = [...groupResults.values()];
    const first = loaded[0];
    if (!first) {
      result = null;
      return;
    }
    result = {
      ...first,
      fields: loaded.flatMap((entry) => entry.fields),
      columns: Object.assign({}, ...loaded.map((entry) => entry.columns)),
    };
  }

  async function loadSeriesGroup(index: number, buckets: number) {
    const group = fieldGroups[index];
    if (!group?.length || inFlightGroups.has(index)) return;
    inFlightGroups.add(index);
    groupLoading.set(index, true);
    groupErrors.delete(index);
    loading = true;
    try {
      reviewController ??= backendState.createSessionReviewController(metadata);
      const groupResult = await reviewController.loadSeries(
        group.map((field) => field.name),
        Math.max(0, Math.round(viewport.startElapsedMs)),
        Math.max(0, Math.round(viewport.endElapsedMs)),
        Math.max(1, Math.min(1_024, Math.round(buckets))),
        backendState.settings?.historyGapThresholdSeconds ?? null
      );
      groupResults.set(index, groupResult);
      rebuildResult();
    } catch (reason) {
      if (!(reason instanceof DOMException && reason.name === 'AbortError')) {
        const message = reason instanceof Error ? reason.message : String(reason);
        groupErrors.set(index, message);
        error = message;
      }
    } finally {
      inFlightGroups.delete(index);
      groupLoading.delete(index);
      loading = [...groupLoading.values()].some(Boolean);
    }
  }

  function observeGroup(node: HTMLElement, index: number) {
    groupNodes.set(index, node);
    groupObserver?.observe(node);
    return {
      destroy() {
        groupObserver?.unobserve(node);
        groupNodes.delete(index);
      },
    };
  }

  async function prefetchGroup(index: number) {
    if (index < 0 || index >= fieldGroups.length) return;
    await loadSeriesGroup(index, Math.max(1, Math.floor(width)));
    if (groupResults.has(index)) await loadSeriesGroup(index + 1, Math.max(1, Math.floor(width)));
  }

  function queueInspection(elapsedMs: number) {
    if (!activeField) return;
    const target = Math.max(0, Math.min(duration, Math.round(elapsedMs)));
    const changed = target !== inspectionMs;
    inspectionMs = target;
    if (changed) inspectionRequestVersion += 1;
    if (!changed && exactStatus === 'exact') return;
    exactStatus = 'pending';
    exactValue = null;
    queuedInspection = target;
    if (inspectionFrame !== null || typeof window === 'undefined') return;
    inspectionFrame = window.requestAnimationFrame(() => {
      inspectionFrame = null;
      scheduleInspectionRequest();
    });
  }

  function scheduleInspectionRequest() {
    if (inspectionInFlight || queuedInspection === null || typeof window === 'undefined') return;
    if (inspectionDebounce !== null) window.clearTimeout(inspectionDebounce);
    inspectionDebounce = window.setTimeout(() => {
      inspectionDebounce = null;
      if (inspectionInFlight || queuedInspection === null) return;
      const target = queuedInspection;
      queuedInspection = null;
      void inspectAt(target, inspectionRequestVersion);
    }, INSPECTION_DEBOUNCE_MS);
  }

  async function inspectAt(target: number, requestVersion: number) {
    if (!activeField) return;
    const field = activeField;
    const activeGroup = fieldGroups.find((group) => group.some((candidate) => candidate.name === field));
    inspectionInFlight = true;
    try {
      reviewController ??= backendState.createSessionReviewController(metadata);
      const response = await reviewController.loadSamples({
        kind: 'samples',
        range: { startElapsedMs: 0, endElapsedMs: duration },
        fields: (activeGroup ?? fields.slice(0, 1)).map((field) => field.name),
        mode: 'nearest',
        limit: 1,
        targetElapsedMs: target,
      });
      if (requestVersion === inspectionRequestVersion && inspectionMs === target && activeField === field) {
        const value = response.values[field]?.[0] ?? null;
        exactValue = value;
        exactStatus = value === null ? 'unavailable' : 'exact';
      }
    } catch (reason) {
      if (!(reason instanceof DOMException && reason.name === 'AbortError')) {
        if (requestVersion === inspectionRequestVersion && inspectionMs === target && activeField === field) {
          exactValue = null;
          exactStatus = 'error';
        }
      }
    } finally {
      inspectionInFlight = false;
      scheduleInspectionRequest();
    }
  }

  function inspectionFieldChanged() {
    inspectionRequestVersion += 1;
    exactStatus = 'pending';
    exactValue = null;
    queuedInspection = inspectionMs;
    scheduleInspectionRequest();
  }

  function elapsedAtClientX(clientX: number, bounds: DOMRect): number {
    const ratio = Math.max(0, Math.min(1, (clientX - bounds.left) / Math.max(1, bounds.width)));
    return visibleViewport.startElapsedMs + ratio * span;
  }

  function reset() {
    previewViewport = null;
    viewport = clampViewport({ startElapsedMs: 0, endElapsedMs: duration }, duration);
  }

  function zoom(factor: number) {
    const center = (visibleViewport.startElapsedMs + visibleViewport.endElapsedMs) / 2;
    const nextSpan = Math.max(1, Math.min(duration, span * factor));
    viewport = clampViewport({ startElapsedMs: center - nextSpan / 2, endElapsedMs: center + nextSpan / 2 }, duration);
    previewViewport = null;
  }

  function beginGesture(event: PointerEvent) {
    if (event.button !== 0 && event.pointerType === 'mouse') return;
    if (!pointers.size && !event.isPrimary) return;
    (event.currentTarget as SVGElement).setPointerCapture?.(event.pointerId);
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const values = [...pointers.values()];
    if (values.length === 1) {
      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
      const start = elapsedAtClientX(event.clientX, bounds);
      gestureStart = { viewport: visibleViewport, distance: 0, ratio: 0.5, pixelWidth: bounds.width };
      selectionStartElapsedMs = start;
      selectionEndElapsedMs = start;
    } else if (values.length === 2) {
      const [first, second] = values;
      const distance = Math.hypot(second!.x - first!.x, second!.y - first!.y);
      const midpoint = (first!.x + second!.x) / 2;
      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (midpoint - bounds.left) / Math.max(1, bounds.width)));
      gestureStart = { viewport: visibleViewport, distance, ratio, pixelWidth: bounds.width };
      selectionStartElapsedMs = null;
      selectionEndElapsedMs = null;
    }
  }

  function moveGesture(event: PointerEvent) {
    if (!pointers.has(event.pointerId) && (event.pointerType === 'mouse' || event.pointerType === 'pen') && event.buttons === 0) {
      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
      queueInspection(
        visibleViewport.startElapsedMs + ((event.clientX - bounds.left) / Math.max(1, bounds.width)) * span
      );
      return;
    }
    if (!pointers.has(event.pointerId) || !gestureStart) return;
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const values = [...pointers.values()];
    if (values.length >= 2) {
      const [first, second] = values;
      previewViewport = pinchViewport(
        gestureStart.viewport,
        Math.max(1, gestureStart.distance),
        Math.hypot(second!.x - first!.x, second!.y - first!.y),
        gestureStart.ratio,
        duration
      );
      selectionStartElapsedMs = null;
      selectionEndElapsedMs = null;
      return;
    }
    const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
    selectionEndElapsedMs = elapsedAtClientX(values[0]!.x, bounds);
  }

  function endGesture(event: PointerEvent) {
    if (event.pointerType === 'touch' || event.pointerType === 'pen') inspectionPinned = true;
    pointers.delete(event.pointerId);
    if (!pointers.size) {
      if (previewViewport) {
        viewport = clampViewport(previewViewport, duration);
      } else if (selectionStartElapsedMs !== null && selectionEndElapsedMs !== null) {
        const start = Math.min(selectionStartElapsedMs, selectionEndElapsedMs);
        const end = Math.max(selectionStartElapsedMs, selectionEndElapsedMs);
        const minimumSpan = (span / Math.max(1, gestureStart?.pixelWidth ?? width)) * MIN_SELECTION_PIXELS;
        if (end - start >= minimumSpan) {
          viewport = clampViewport({ startElapsedMs: start, endElapsedMs: end }, duration);
        } else {
          queueInspection(selectionEndElapsedMs);
        }
      }
      selectionStartElapsedMs = null;
      selectionEndElapsedMs = null;
      previewViewport = null;
      gestureStart = null;
    }
  }

  function cancelGesture(event: PointerEvent) {
    pointers.delete(event.pointerId);
    if (!pointers.size) {
      selectionStartElapsedMs = null;
      selectionEndElapsedMs = null;
      previewViewport = null;
      gestureStart = null;
    }
  }

  function path(column: SessionSeriesResult['columns'][string]): string[] {
    return buildPathSegments(column, { width: CHART_WIDTH, height: CHART_HEIGHT, viewport: visibleViewport });
  }

  function dismissInspection() {
    inspectionRequestVersion += 1;
    queuedInspection = null;
    if (inspectionDebounce !== null && typeof window !== 'undefined') {
      window.clearTimeout(inspectionDebounce);
      inspectionDebounce = null;
    }
    inspectionPinned = false;
    exactStatus = 'idle';
    exactValue = null;
  }

  onMount(() => {
    if (!plot) return;
    const dismissOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && inspectionPinned) dismissInspection();
    };
    window.addEventListener('keydown', dismissOnEscape);
    const observer = new ResizeObserver((entries) => {
      const next = Math.max(0, Math.floor(entries[0]?.contentRect.width ?? 0));
      if (next <= 0 || next === width) return;
      if (resizeFrame !== null && typeof window !== 'undefined') window.cancelAnimationFrame(resizeFrame);
      const commit = () => {
        resizeFrame = null;
        width = next;
      };
      resizeFrame = typeof window === 'undefined' ? null : window.requestAnimationFrame(commit);
      if (resizeFrame === null) commit();
    });
    observer.observe(plot);
    groupObserver = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        const index = Number((entry.target as HTMLElement).dataset.historyGroup);
        if (Number.isInteger(index)) prefetchGroup(index);
      });
    }, { root: plot, rootMargin: '160px' });
    groupNodes.forEach((node) => groupObserver?.observe(node));
    return () => {
      window.removeEventListener('keydown', dismissOnEscape);
      observer.disconnect();
      groupObserver?.disconnect();
      groupObserver = null;
    };
  });

  onDestroy(() => {
    if (inspectionFrame !== null && typeof window !== 'undefined') window.cancelAnimationFrame(inspectionFrame);
    if (inspectionDebounce !== null && typeof window !== 'undefined') window.clearTimeout(inspectionDebounce);
    if (resizeFrame !== null && typeof window !== 'undefined') window.cancelAnimationFrame(resizeFrame);
    groupObserver?.disconnect();
    reviewController?.dispose();
  });
</script>

<section
  bind:this={plot}
  class="relative mt-5 rounded-lg border border-outline p-4"
  aria-labelledby="history-heading"
>
  <div class="flex flex-wrap items-center justify-between gap-2">
    <h3 id="history-heading" class="font-medium">Historical telemetry</h3>
    <div class="flex gap-2">
      <button
        type="button"
        class="touch-manipulation rounded bg-elevated px-2 py-1 text-xs"
        onpointerdown={(event) => activateFromPointer(event, () => zoom(0.5))}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => zoom(0.5))}>Zoom in</button
      >
      <button
        type="button"
        class="touch-manipulation rounded bg-elevated px-2 py-1 text-xs"
        onpointerdown={(event) => activateFromPointer(event, () => zoom(2))}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => zoom(2))}>Zoom out</button
      >
      <button
        type="button"
        class="touch-manipulation rounded bg-elevated px-2 py-1 text-xs"
        onpointerdown={(event) => activateFromPointer(event, reset)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, reset)}>Reset</button
      >
      <button
        type="button"
        class="touch-manipulation rounded bg-elevated px-2 py-1 text-xs"
        onpointerdown={(event) =>
          activateFromPointer(event, () => {
            viewport = panViewport(viewport, -span * 0.25, duration);
          })}
        onclick={(event) =>
          activateFromKeyboardOrAssistiveClick(event, () => {
            viewport = panViewport(viewport, -span * 0.25, duration);
          })}>Earlier</button
      >
      <button
        type="button"
        class="touch-manipulation rounded bg-elevated px-2 py-1 text-xs"
        onpointerdown={(event) =>
          activateFromPointer(event, () => {
            viewport = panViewport(viewport, span * 0.25, duration);
          })}
        onclick={(event) =>
          activateFromKeyboardOrAssistiveClick(event, () => {
            viewport = panViewport(viewport, span * 0.25, duration);
          })}>Later</button
      >
    </div>
  </div>
  <div class="mt-3 flex flex-wrap items-center gap-2 text-xs">
    <label for="history-inspection-field">Inspect</label>
    <select id="history-inspection-field" bind:value={activeField} onchange={inspectionFieldChanged} class="rounded bg-elevated px-2 py-1">
      {#each fields as field (field.name)}<option value={field.name}>{field.name}</option>{/each}
    </select>
    <input
      id="history-inspection-range"
      class="min-w-44 flex-1"
      type="range"
      min="0"
      max={duration}
      step="1"
      bind:value={inspectionMs}
      aria-label="Inspect elapsed time"
      aria-valuetext={`${inspectionMs} milliseconds elapsed`}
      oninput={(event) => queueInspection(Number((event.currentTarget as HTMLInputElement).value))}
    />
    <span aria-live="polite">
      {#if exactStatus === 'pending'}Loading exact value…
      {:else if exactStatus === 'exact'}{activeField}: {exactValue}
      {:else if exactStatus === 'unavailable'}{activeField}: unavailable
      {:else if exactStatus === 'error'}Exact value unavailable
      {:else}No exact value selected{/if}
    </span>
  </div>
  {#if inspectionVisible}
    <div
      class="pointer-events-none absolute right-4 top-20 z-10 rounded bg-surface px-2 py-1 text-xs shadow"
      style={`max-width: min(16rem, calc(100% - 2rem)); left: ${Math.max(4, Math.min(76, ((inspectionMs - visibleViewport.startElapsedMs) / span) * 100))}%`}
      role="status"
    >
      <span>{inspectionMs} ms</span>
      {#if exactStatus === 'pending'} · Loading exact value…
      {:else if exactStatus === 'exact'} · {activeField}: {exactValue}{#if fields.find((field) => field.name === activeField)?.unit} {fields.find((field) => field.name === activeField)?.unit}{/if}
      {:else if exactStatus === 'unavailable'} · {activeField}: unavailable
      {:else if exactStatus === 'error'} · Exact value unavailable{/if}
      {#if inspectionPinned}<button type="button" class="pointer-events-auto ml-1 underline" onclick={dismissInspection}>Close</button>{/if}
    </div>
  {/if}
  {#if loading}<p class="mt-2 text-xs text-muted" role="status">Loading chart data…</p>{/if}
  {#if error}<p class="mt-2 text-sm text-warning" role="alert">{error}</p>{/if}
  {#if result}
    <div class="mt-3 space-y-3">
      {#each fieldGroups as group, groupIndex (groupIndex)}
        <div use:observeGroup={groupIndex} data-history-group={groupIndex} class="space-y-3">
          {#if groupLoading.get(groupIndex) && !groupResults.has(groupIndex)}
            <p class="text-xs text-muted" role="status">Loading group {groupIndex + 1}…</p>
          {/if}
          {#if groupErrors.get(groupIndex)}
            <p class="text-xs text-warning" role="alert">{groupErrors.get(groupIndex)}</p>
          {/if}
          {#each group as field (field.name)}
            {@const column = result?.columns[field.name]}
            {@const paths = column ? path(column) : []}
            {@const domain = column ? finiteDomain(column.values) : null}
            {@const yTicks = axisTicks(domain, Y_TICK_COUNT)}
            {@const zero = zeroReference(domain)}
            <div>
              <div class="flex items-baseline justify-between gap-2 text-xs">
                <span>{field.name}</span><span class="text-muted">{field.unit ?? 'value'}</span>
              </div>
              {#if paths.length}
                <div class="mt-1 flex items-stretch gap-1">
                  <div class="flex h-24 w-12 shrink-0 flex-col justify-between text-right text-[8px] leading-none text-muted" aria-hidden="true">
                    {#each yTicks as tick}<span>{formatAxisTick(tick)}</span>{/each}
                  </div>
                <svg
                  class="h-24 min-w-0 flex-1 touch-pan-y rounded bg-canvas"
                  viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
                  preserveAspectRatio="none"
                  role="img"
                  aria-label={`${field.name} over elapsed time`}
                  onpointerdown={(event) => {
                    beginGesture(event);
                    if (event.pointerType === 'touch' || event.pointerType === 'pen') {
                      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
                      queueInspection(elapsedAtClientX(event.clientX, bounds));
                    }
                  }}
                  onpointermove={moveGesture}
                  onpointerup={endGesture}
                  onpointercancel={cancelGesture}
                  onpointerleave={() => {
                    if (!inspectionPinned) dismissInspection();
                  }}
                >
                  {#each yTicks as tick}
                    {@const y = CHART_HEIGHT - ((tick - (domain?.min ?? 0)) / Math.max(Number.EPSILON, (domain?.max ?? 1) - (domain?.min ?? 0))) * (CHART_HEIGHT - 4) - 2}
                    <line x1="0" x2={CHART_WIDTH} y1={y} y2={y} stroke="var(--app-muted)" stroke-opacity="0.35" stroke-dasharray="2 2" />
                  {/each}
                  {#if zero !== null}<line x1="0" x2={CHART_WIDTH} y1={CHART_HEIGHT - zero * (CHART_HEIGHT - 4) - 2} y2={CHART_HEIGHT - zero * (CHART_HEIGHT - 4) - 2} stroke="var(--app-muted)" stroke-dasharray="2 2" />{/if}
                  {#each paths as segment (segment)}<path
                      d={segment}
                      fill="none"
                      stroke="var(--app-accent)"
                      stroke-width="1.5"
                      vector-effect="non-scaling-stroke"
                    />{/each}
                  {#if inspectionVisible}
                    <line
                      x1={((inspectionMs - visibleViewport.startElapsedMs) / span) * CHART_WIDTH}
                      x2={((inspectionMs - visibleViewport.startElapsedMs) / span) * CHART_WIDTH}
                      y1="0"
                      y2={CHART_HEIGHT}
                      stroke="var(--app-accent)"
                      stroke-dasharray="3 2"
                      vector-effect="non-scaling-stroke"
                    />
                  {/if}
                  {#if selectionRange}
                    <rect x="0" y="0" width={selectionRange.leftX} height={CHART_HEIGHT} fill="var(--app-surface)" fill-opacity="0.82" />
                    <rect x={selectionRange.rightX} y="0" width={CHART_WIDTH - selectionRange.rightX} height={CHART_HEIGHT} fill="var(--app-surface)" fill-opacity="0.82" />
                    <line x1={selectionRange.startX} x2={selectionRange.startX} y1="0" y2={CHART_HEIGHT} stroke="var(--app-accent)" stroke-width="1.5" />
                    <line x1={selectionRange.endX} x2={selectionRange.endX} y1="0" y2={CHART_HEIGHT} stroke="var(--app-accent)" stroke-width="1.5" />
                  {/if}
                </svg>
                </div>
                {#if column.values.some((value) => value === null)}<p class="text-[10px] text-muted">Null or time gaps are shown as breaks.</p>{/if}
              {:else if column}<p class="mt-1 text-xs text-muted">No finite samples in this range.</p>
              {:else}<p class="mt-1 text-xs text-muted">Waiting for this field group.</p>{/if}
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {:else if !loading}<p class="mt-2 text-sm text-muted">No retained numeric history is available.</p>{/if}
</section>
