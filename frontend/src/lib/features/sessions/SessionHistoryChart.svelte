<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import { MAX_SESSION_DATA_FIELDS, type SessionMetadata } from '$lib/backend/contract';
  import type { SessionReviewController } from '$lib/backend/session-controller';
  import type { SessionSeriesColumn, SessionSeriesResult } from '$lib/backend/session-data';
  import { backendState } from '$lib/backend/state.svelte';
  import { reportFrontendPerformance } from '$lib/backend/performance-log';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { axisTicks, buildPathSegments, finiteDomain, zeroReference, type NumericDomain } from './history-geometry';
  import { buildChartTooltipEntries } from './chart-tooltip';
  import {
    formatSessionFieldValue,
    sessionChartGroups,
    sessionFieldIsChartable,
    sessionFieldUnit,
    sessionChartField,
    sortSessionFields,
    type SessionChartGroup,
  } from './session-fields';
  import { firstVisibleChartField, toggleChartField } from './chart-visibility';
  import {
    ExactTooltipController,
    formatElapsedForAssistive,
    type ExactTooltipSnapshot,
    type ExactTooltipValue,
  } from './exact-tooltip';
  import { clampViewport, panViewport, pinchViewport, type Viewport } from './viewport';
  import SectionTitle from '../../ui/SectionTitle.svelte';
  import Spinner from '../../ui/Spinner.svelte';

  let {
    metadata,
    onselectionchange = () => {},
  }: { metadata: SessionMetadata; onselectionchange?: (elapsedMs: number) => void } = $props();
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
  let hiddenFields = new SvelteSet<string>();
  // Keep the in-flight guard outside Svelte's reactive graph. The initial
  // loading effect must not rerun just because it updates the loading map.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const inFlightGroups = new Set<number>();
  let groupNodes = new SvelteMap<number, HTMLElement>();
  let groupObserver: IntersectionObserver | null = null;
  let resizeFrame: number | null = null;
  let reviewController: SessionReviewController | null = null;
  let activeField = $state('');
  let inspectionMs = $state(0);
  let exactStatus = $state<ExactTooltipSnapshot['status']>('idle');
  let exactValue = $state<ExactTooltipValue | null>(null);
  let exactController: ExactTooltipController | null = null;
  let inspectionPinned = $state(false);
  let inspectionFrame: number | null = null;
  let queuedInspection: number | null = null;
  let touchDevice = $state(false);
  let mouseDevice = $state(false);
  let longTaskObserver: PerformanceObserver | null = null;
  let longTaskCount = 0;
  let longTaskTotalMs = 0;
  let longTaskMaxMs = 0;

  const CHART_WIDTH = 320;
  const CHART_HEIGHT = 96;
  const Y_TICK_COUNT = 5;
  const MIN_SELECTION_PIXELS = 8;
  const INTERACTION_MODES = ['info', 'zoom', 'pan'] as const;
  const MAX_FIELDS_PER_SERIES_REQUEST = MAX_SESSION_DATA_FIELDS;
  const FIRST_VISIBLE_GROUP_FIELDS = 9;

  type InteractionMode = (typeof INTERACTION_MODES)[number];
  let interactionMode = $state<InteractionMode>('info');

  const fields = $derived(sortSessionFields(metadata.fields.filter((field) => sessionFieldIsChartable(field.name))));
  const fieldGroups = $derived.by(() => {
    const groups: SessionMetadata['fields'][] = [];
    let stable: SessionMetadata['fields'] = [];
    fields.forEach((field) => {
      if (field.source === 'stable' && stable.length < FIRST_VISIBLE_GROUP_FIELDS) {
        stable = [...stable, field];
        return;
      }
      if (!groups.length || groups.at(-1)!.length >= MAX_FIELDS_PER_SERIES_REQUEST) groups.push([]);
      groups[groups.length - 1] = [...groups.at(-1)!, field];
    });
    if (stable.length) groups.unshift(stable);
    return groups;
  });
  const chartGroups = $derived(sessionChartGroups(fields));
  const visibleViewport = $derived(previewViewport ?? viewport);
  const span = $derived(Math.max(1, visibleViewport.endElapsedMs - visibleViewport.startElapsedMs));
  const duration = $derived(Math.max(1, metadata.summary.totalElapsedMs));
  const isZoomed = $derived(viewport.endElapsedMs - viewport.startElapsedMs < duration);
  let pointers = new SvelteMap<number, { x: number; y: number }>();
  let gestureStart: {
    viewport: Viewport;
    distance: number;
    ratio: number;
    pixelWidth: number;
    startElapsedMs: number;
  } | null = null;
  let gestureMode = $state<InteractionMode | null>(null);
  let gestureStartedAt: number | null = null;
  let gesturePreviewReported = false;
  let selectionStartElapsedMs = $state<number | null>(null);
  let selectionEndElapsedMs = $state<number | null>(null);
  let panEdge = $state<'start' | 'end' | null>(null);
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
  let inspectionVisible = $derived(exactStatus !== 'idle');

  $effect(() => {
    if (!activeField && fields.length) {
      const firstVisible = fields.find((field) => !hiddenFields.has(field.name));
      if (firstVisible) activeField = firstVisible.name;
    }
    if (!width || !fieldGroups.length) return;
    void loadSeriesGroup(0, width);
  });

  function chartGroupRequestIndex(chartGroup: SessionChartGroup): number {
    const index = fieldGroups.findIndex((group) =>
      chartGroup.fields.some((field) => group.some((candidate) => candidate.name === field.name))
    );
    return index < 0 ? 0 : index;
  }

  function chartGroupsForRequestGroup(index: number): SessionChartGroup[] {
    return chartGroups.filter((chartGroup) => chartGroupRequestIndex(chartGroup) === index);
  }

  function chartDomain(chartGroup: SessionChartGroup) {
    const values = chartGroup.fields.flatMap((field) => result?.columns[field.name]?.values ?? []);
    return finiteDomain(values, chartGroup.axis);
  }

  function chartTooltipEntries(chartGroup: SessionChartGroup, domain: NumericDomain) {
    if (exactStatus === 'pending' || exactStatus === 'unavailable') return [];
    let columns = result?.columns;
    const exact = exactValue;
    if (exact && exactStatus !== 'idle') {
      const exactColumns: Record<string, SessionSeriesColumn> = {};
      Object.entries(exact.values).forEach(([field, value]) => {
        exactColumns[field] = {
          elapsedMs: [exact.elapsedMs],
          sequence: [0],
          values: [value],
          generations: [0],
        };
      });
      columns = exactColumns;
    }
    return buildChartTooltipEntries(chartGroup, columns, hiddenFields, exact?.elapsedMs ?? inspectionMs, domain, {
      width: CHART_WIDTH,
      height: CHART_HEIGHT,
      viewport: visibleViewport,
    });
  }

  const chartModels = $derived.by(() => {
    const models = new SvelteMap<
      string,
      {
        domain: NumericDomain | null;
        hasColumns: boolean;
        hasFiniteValues: boolean;
        yTicks: number[];
        yTickLabels: number[];
        zero: number | null;
        paths: Map<string, string[]>;
      }
    >();

    chartGroups.forEach((chartGroup) => {
      const domain = chartDomain(chartGroup);
      const paths = new SvelteMap<string, string[]>();
      if (domain) {
        chartGroup.fields.forEach((field) => {
          const column = result?.columns[field.name];
          if (column) {
            paths.set(
              field.name,
              buildPathSegments(column, {
                width: CHART_WIDTH,
                height: CHART_HEIGHT,
                viewport: visibleViewport,
                domain,
              })
            );
          }
        });
      }
      const yTicks = axisTicks(domain, Y_TICK_COUNT);
      models.set(chartGroup.id, {
        domain,
        hasColumns: chartGroup.fields.some((field) => Boolean(result?.columns[field.name])),
        hasFiniteValues: chartGroup.fields.some((field) =>
          result?.columns[field.name]?.values.some((value) => value !== null && Number.isFinite(value))
        ),
        yTicks,
        yTickLabels: [...yTicks].reverse(),
        zero: zeroReference(domain),
        paths,
      });
    });
    return models;
  });

  function isFieldVisible(fieldName: string): boolean {
    return !hiddenFields.has(fieldName);
  }

  function toggleField(fieldName: string): void {
    const willHide = !hiddenFields.has(fieldName);
    const nextHidden = toggleChartField(hiddenFields, fieldName);
    hiddenFields.clear();
    nextHidden.forEach((name) => hiddenFields.add(name));

    if (willHide && activeField === fieldName) {
      dismissInspection(true);
      const group = chartGroups.find((candidate) => candidate.fields.some((field) => field.name === fieldName));
      activeField = group ? (firstVisibleChartField(group.fields, hiddenFields) ?? '') : '';
    } else if (!willHide && !activeField) {
      activeField = fieldName;
    }
  }

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
    const started = globalThis.performance?.now() ?? Date.now();
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
      const paintStarted = globalThis.performance?.now() ?? Date.now();
      requestAnimationFrame(() => {
        const groupNode = plot?.querySelector<HTMLElement>(`[data-history-group="${index}"]`);
        reportFrontendPerformance('session-history-group', {
          group: index,
          fields: group.length,
          buckets,
          responseAndStateMs: Math.round((paintStarted - started) * 100) / 100,
          firstPaintMs: Math.round(((globalThis.performance?.now() ?? Date.now()) - started) * 100) / 100,
          observations: Object.values(groupResult.columns).reduce((sum, column) => sum + column.values.length, 0),
          svgCount: groupNode?.querySelectorAll('svg').length ?? 0,
          svgPathSegments: groupNode?.querySelectorAll('path').length ?? 0,
          cacheEntries: reviewController?.cacheStats.entries ?? 0,
          cacheBytes: reviewController?.cacheStats.bytes ?? 0,
          longTaskCount,
          longTaskTotalMs: Math.round(longTaskTotalMs * 100) / 100,
          longTaskMaxMs: Math.round(longTaskMaxMs * 100) / 100,
        });
      });
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

  function queueInspection(elapsedMs: number, pinned = false) {
    if (!activeField) return;
    const target = Math.max(0, Math.min(duration, Math.round(elapsedMs)));
    inspectionPinned = pinned;
    queuedInspection = target;
    if (inspectionFrame !== null) return;
    if (typeof window === 'undefined') {
      commitInspection();
      return;
    }
    inspectionFrame = window.requestAnimationFrame(() => {
      inspectionFrame = null;
      commitInspection();
    });
  }

  function commitInspection() {
    if (queuedInspection === null) return;
    const target = queuedInspection;
    queuedInspection = null;
    if (target === inspectionMs && exactStatus === 'exact') return;
    inspectionMs = target;
    onselectionchange(target);
    exactValue = null;
    if (!activeField) return;
    exactController ??= new ExactTooltipController(
      async (targetElapsedMs) => {
        reviewController ??= backendState.createSessionReviewController(metadata);
        const chartGroup = chartGroups.find((candidate) =>
          candidate.fields.some((field) => field.name === activeField)
        );
        if (!chartGroup) return null;
        const sample = await reviewController.loadSamples({
          kind: 'samples',
          range: {
            startElapsedMs: Math.max(0, Math.round(viewport.startElapsedMs)),
            endElapsedMs: Math.max(0, Math.round(viewport.endElapsedMs)),
          },
          fields: chartGroup.fields.map((field) => field.name),
          mode: 'nearest',
          limit: 1,
          cursor: null,
          targetElapsedMs,
        });
        if (!sample.elapsedMs.length) return null;
        return {
          elapsedMs: sample.elapsedMs[0]!,
          values: Object.fromEntries(
            chartGroup.fields.map((field) => [field.name, sample.values[field.name]?.[0] ?? null])
          ),
        };
      },
      (snapshot) => {
        exactStatus = snapshot.status;
        exactValue = snapshot.value;
        inspectionMs = snapshot.targetElapsedMs;
        onselectionchange(snapshot.targetElapsedMs);
        inspectionPinned = snapshot.pinned;
      }
    );
    exactController.inspect(target, activeField, inspectionPinned);
  }

  function elapsedAtClientX(clientX: number, bounds: DOMRect): number {
    const ratio = Math.max(0, Math.min(1, (clientX - bounds.left) / Math.max(1, bounds.width)));
    return visibleViewport.startElapsedMs + ratio * span;
  }

  function reset() {
    previewViewport = null;
    viewport = clampViewport({ startElapsedMs: 0, endElapsedMs: duration }, duration);
  }

  function isTouchPointer(event: PointerEvent): boolean {
    return event.pointerType === 'touch';
  }

  function resetGesture() {
    pointers.clear();
    selectionStartElapsedMs = null;
    selectionEndElapsedMs = null;
    previewViewport = null;
    gestureStart = null;
    gestureMode = null;
    gestureStartedAt = null;
    gesturePreviewReported = false;
    panEdge = null;
  }

  function reportGesturePreview() {
    if (gesturePreviewReported || gestureStartedAt === null) return;
    gesturePreviewReported = true;
    reportFrontendPerformance('session-history-gesture-preview', {
      mode: gestureMode ?? interactionMode,
      previewMs: Math.round(((globalThis.performance?.now() ?? Date.now()) - gestureStartedAt) * 100) / 100,
      svgPathSegments: plot?.querySelectorAll('path').length ?? 0,
    });
  }

  function nextInteractionMode(): InteractionMode {
    const index = INTERACTION_MODES.indexOf(interactionMode);
    return INTERACTION_MODES[(index + 1) % INTERACTION_MODES.length]!;
  }

  function cycleInteractionMode() {
    interactionMode = nextInteractionMode();
    resetGesture();
    dismissInspection(true);
  }

  function preventChartContextMenu(event: MouseEvent) {
    if (mouseDevice) event.preventDefault();
  }

  function beginGesture(event: PointerEvent) {
    const rightMousePan = mouseDevice && event.pointerType === 'mouse' && event.button === 2;
    if (event.pointerType === 'mouse' && event.button !== 0 && !rightMousePan) return;
    if (!pointers.size && !event.isPrimary) return;
    if (rightMousePan) event.preventDefault();
    (event.currentTarget as SVGElement).setPointerCapture?.(event.pointerId);
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const values = [...pointers.values()];
    const touchPointer = isTouchPointer(event);
    const mode = rightMousePan ? 'pan' : interactionMode;
    gestureMode = mode;
    gestureStartedAt ??= globalThis.performance?.now() ?? Date.now();
    panEdge = null;
    if (rightMousePan) dismissInspection(true);
    if (touchPointer && mode === 'info') {
      if (values.length === 1) {
        const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
        queueInspection(elapsedAtClientX(event.clientX, bounds), true);
      } else {
        pointers.delete(event.pointerId);
      }
      gestureStart = null;
      selectionStartElapsedMs = null;
      selectionEndElapsedMs = null;
      return;
    }
    if (touchPointer && mode === 'pan' && values.length > 1) {
      pointers.delete(event.pointerId);
      return;
    }
    if (values.length === 1) {
      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
      const start = elapsedAtClientX(event.clientX, bounds);
      gestureStart = {
        viewport: visibleViewport,
        distance: 0,
        ratio: 0.5,
        pixelWidth: bounds.width,
        startElapsedMs: start,
      };
      if (mode === 'zoom') {
        selectionStartElapsedMs = start;
        selectionEndElapsedMs = start;
      } else {
        selectionStartElapsedMs = null;
        selectionEndElapsedMs = null;
      }
    } else if (values.length === 2) {
      const [first, second] = values;
      const distance = Math.hypot(second!.x - first!.x, second!.y - first!.y);
      const midpoint = (first!.x + second!.x) / 2;
      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (midpoint - bounds.left) / Math.max(1, bounds.width)));
      gestureStart = {
        viewport: visibleViewport,
        distance,
        ratio,
        pixelWidth: bounds.width,
        startElapsedMs: visibleViewport.startElapsedMs + ratio * span,
      };
      selectionStartElapsedMs = null;
      selectionEndElapsedMs = null;
    }
  }

  function moveGesture(event: PointerEvent) {
    if (
      !pointers.has(event.pointerId) &&
      (event.pointerType === 'mouse' || event.pointerType === 'pen') &&
      event.buttons === 0
    ) {
      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
      queueInspection(
        visibleViewport.startElapsedMs + ((event.clientX - bounds.left) / Math.max(1, bounds.width)) * span
      );
      return;
    }
    if (!pointers.has(event.pointerId)) return;
    const mode = gestureMode ?? interactionMode;
    if (isTouchPointer(event) && mode === 'info') {
      const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
      queueInspection(elapsedAtClientX(event.clientX, bounds), true);
      return;
    }
    if (!gestureStart) return;
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const values = [...pointers.values()];
    if (values.length >= 2 && (!isTouchPointer(event) || mode === 'zoom')) {
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
      reportGesturePreview();
      return;
    }
    const bounds = (event.currentTarget as SVGElement).getBoundingClientRect();
    if (mode === 'pan') {
      const currentElapsedMs = elapsedAtClientX(values[0]!.x, bounds);
      const requestedDelta = gestureStart.startElapsedMs - currentElapsedMs;
      previewViewport = panViewport(gestureStart.viewport, requestedDelta, duration);
      panEdge =
        requestedDelta < 0 && previewViewport.startElapsedMs === 0
          ? 'start'
          : requestedDelta > 0 && previewViewport.endElapsedMs === duration
            ? 'end'
            : null;
      reportGesturePreview();
      return;
    }
    if (mode === 'zoom') {
      selectionEndElapsedMs = elapsedAtClientX(values[0]!.x, bounds);
      reportGesturePreview();
    }
  }

  function endGesture(event: PointerEvent) {
    const mode = gestureMode ?? interactionMode;
    pointers.delete(event.pointerId);
    if (!pointers.size) {
      if (previewViewport) {
        viewport = clampViewport(previewViewport, duration);
      } else if (mode === 'zoom' && selectionStartElapsedMs !== null && selectionEndElapsedMs !== null) {
        const start = Math.min(selectionStartElapsedMs, selectionEndElapsedMs);
        const end = Math.max(selectionStartElapsedMs, selectionEndElapsedMs);
        const minimumSpan = (span / Math.max(1, gestureStart?.pixelWidth ?? width)) * MIN_SELECTION_PIXELS;
        if (end - start >= minimumSpan) {
          viewport = clampViewport({ startElapsedMs: start, endElapsedMs: end }, duration);
        } else if (!isTouchPointer(event)) {
          queueInspection(selectionEndElapsedMs);
        }
      }
      resetGesture();
    }
  }

  function cancelGesture(event: PointerEvent) {
    const mode = gestureMode ?? interactionMode;
    pointers.delete(event.pointerId);
    if (!pointers.size) {
      resetGesture();
      if (isTouchPointer(event) && mode === 'info') dismissInspection(true);
    }
  }

  function tooltipStyle(pointX: number): string {
    const alignLeft = pointX > CHART_WIDTH / 2;
    const left = `${(Math.max(0, Math.min(CHART_WIDTH, pointX)) / CHART_WIDTH) * 100}%`;
    return `left: ${left}; transform: ${alignLeft ? 'translateX(calc(-100% - 0.25rem))' : 'translateX(0.25rem)'}`;
  }

  function formatTooltipValue(fieldName: string, value: number): string {
    return formatSessionFieldValue(fieldName, value);
  }

  function dismissInspection(force = false) {
    if (inspectionPinned && !force) return;
    queuedInspection = null;
    if (inspectionFrame !== null && typeof window !== 'undefined') {
      window.cancelAnimationFrame(inspectionFrame);
      inspectionFrame = null;
    }
    exactStatus = 'idle';
    exactValue = null;
    inspectionPinned = false;
    exactController?.dismiss();
  }

  function inspectFromSlider(event: Event) {
    const target = Number((event.currentTarget as HTMLInputElement).value);
    if (!Number.isFinite(target)) return;
    queueInspection(target, true);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && exactStatus !== 'idle') {
      event.preventDefault();
      dismissInspection(true);
    }
  }

  onMount(() => {
    if (!plot) return;
    const touchMedia = window.matchMedia('(hover: none) and (pointer: coarse)');
    const updateTouchDevice = () => {
      touchDevice = touchMedia.matches || navigator.maxTouchPoints > 0 || 'ontouchstart' in window;
    };
    const mouseMedia = window.matchMedia('(hover: hover) and (pointer: fine)');
    const updateMouseDevice = () => {
      mouseDevice = mouseMedia.matches;
    };
    updateTouchDevice();
    updateMouseDevice();
    touchMedia.addEventListener('change', updateTouchDevice);
    mouseMedia.addEventListener('change', updateMouseDevice);
    if (typeof PerformanceObserver !== 'undefined' && PerformanceObserver.supportedEntryTypes?.includes('longtask')) {
      longTaskObserver = new PerformanceObserver((list) => {
        list.getEntries().forEach((entry) => {
          if (longTaskCount >= 64) return;
          longTaskCount += 1;
          longTaskTotalMs += entry.duration;
          longTaskMaxMs = Math.max(longTaskMaxMs, entry.duration);
          reportFrontendPerformance('session-history-longtask', {
            durationMs: Math.round(entry.duration * 100) / 100,
            count: longTaskCount,
          });
        });
      });
      longTaskObserver.observe({ type: 'longtask', buffered: true });
    }
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
    groupObserver = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) return;
          const index = Number((entry.target as HTMLElement).dataset.historyGroup);
          if (Number.isInteger(index)) prefetchGroup(index);
        });
      },
      // `plot` grows with the chart groups and is not a scroll container. If
      // used as the root, every group is intersecting as soon as the page is
      // rendered and the whole session is fetched immediately. The viewport
      // root keeps the first group eager while limiting prefetch to nearby
      // groups.
      { rootMargin: '160px' }
    );
    groupNodes.forEach((node) => groupObserver?.observe(node));
    return () => {
      touchMedia.removeEventListener('change', updateTouchDevice);
      mouseMedia.removeEventListener('change', updateMouseDevice);
      observer.disconnect();
      longTaskObserver?.disconnect();
      longTaskObserver = null;
      groupObserver?.disconnect();
      groupObserver = null;
    };
  });

  onDestroy(() => {
    if (inspectionFrame !== null && typeof window !== 'undefined') window.cancelAnimationFrame(inspectionFrame);
    if (resizeFrame !== null && typeof window !== 'undefined') window.cancelAnimationFrame(resizeFrame);
    longTaskObserver?.disconnect();
    groupObserver?.disconnect();
    exactController?.dispose();
    reviewController?.dispose();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div bind:this={plot} class="mt-3">
  {#if loading}
    <p class="mt-2 flex flex-row items-center gap-2 text-lg text-muted" role="status">
      <Spinner />
      Loading chart data…
    </p>
  {/if}

  {#if error}<p class="mt-2 text-sm text-warning" role="alert">{error}</p>{/if}

  {#if result}
    <div class="mt-3 space-y-3">
      <div class="flex items-center gap-2 text-xs text-muted">
        <label for="session-crosshair-position" class="shrink-0">Inspect elapsed time</label>
        <input
          id="session-crosshair-position"
          class="min-w-0 flex-1 accent-accent"
          type="range"
          min="0"
          max={duration}
          step="1"
          value={inspectionMs}
          aria-label="Inspect elapsed time"
          aria-valuetext={formatElapsedForAssistive(inspectionMs, duration)}
          oninput={inspectFromSlider}
        />
        <button
          type="button"
          class="shrink-0 rounded px-1 text-xs text-muted underline"
          aria-label="Close chart inspection"
          onclick={() => dismissInspection(true)}>Close</button
        >
      </div>
      {#each fieldGroups as group, groupIndex (groupIndex)}
        <div
          use:observeGroup={groupIndex}
          data-history-group={groupIndex}
          data-history-field-count={group.length}
          class="space-y-3"
        >
          {#if groupLoading.get(groupIndex) && !groupResults.has(groupIndex)}
            <p class="text-xs text-muted" role="status">Loading group {groupIndex + 1}…</p>
          {/if}

          {#if groupErrors.get(groupIndex)}
            <p class="text-xs text-warning" role="alert">{groupErrors.get(groupIndex)}</p>
          {/if}

          {#each chartGroupsForRequestGroup(groupIndex) as chartGroup (chartGroup.id)}
            {@const chartModel = chartModels.get(chartGroup.id)}
            {@const domain = chartModel?.domain ?? null}
            {@const hasColumns = chartModel?.hasColumns ?? false}
            {@const hasFiniteValues = chartModel?.hasFiniteValues ?? false}
            {@const yTicks = chartModel?.yTicks ?? []}
            {@const yTickLabels = chartModel?.yTickLabels ?? []}
            {@const zero = chartModel?.zero ?? null}
            {@const inspectionGroupActive = chartGroup.fields.some((field) => field.name === activeField)}
            {@const tooltipEntries =
              inspectionVisible && inspectionGroupActive && domain ? chartTooltipEntries(chartGroup, domain) : []}
            {@const tooltipAnchor = tooltipEntries[0]?.point ?? null}
            {@const tooltipX =
              tooltipAnchor?.x ?? ((inspectionMs - visibleViewport.startElapsedMs) / span) * CHART_WIDTH}
            <div>
              <SectionTitle title={chartGroup.title} detail={chartGroup.unit ?? 'value'} />

              {#if hasColumns && domain && hasFiniteValues}
                <div class="mt-2 flex items-stretch gap-1">
                  <div
                    class="flex h-36 shrink-0 flex-col justify-between text-right text-[8px] leading-none text-muted"
                    aria-hidden="true"
                  >
                    {#each yTickLabels as tick (tick)}
                      <span>{formatSessionFieldValue(chartGroup.fields[0]!.name, tick)}</span>
                    {/each}
                  </div>
                  <div class="relative min-w-0 flex-1">
                    <svg
                      class="block h-36 w-full touch-pan-y rounded bg-canvas"
                      viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
                      preserveAspectRatio="none"
                      role="img"
                      aria-label={`${chartGroup.title} over elapsed time`}
                      onpointerdown={(event) => {
                        beginGesture(event);
                      }}
                      onpointermove={moveGesture}
                      onpointerup={endGesture}
                      onpointercancel={cancelGesture}
                      oncontextmenu={preventChartContextMenu}
                      onpointerleave={() => {
                        dismissInspection();
                      }}
                    >
                      {#each yTicks as tick (tick)}
                        {@const y =
                          CHART_HEIGHT -
                          ((tick - domain.min) / Math.max(Number.EPSILON, domain.max - domain.min)) *
                            (CHART_HEIGHT - 4) -
                          2}
                        <line
                          x1="0"
                          x2={CHART_WIDTH}
                          y1={y}
                          y2={y}
                          stroke="var(--app-muted)"
                          stroke-opacity="0.35"
                          stroke-dasharray="2 2"
                        />
                      {/each}
                      {#if zero !== null}
                        <line
                          x1="0"
                          x2={CHART_WIDTH}
                          y1={CHART_HEIGHT - zero * (CHART_HEIGHT - 4) - 2}
                          y2={CHART_HEIGHT - zero * (CHART_HEIGHT - 4) - 2}
                          stroke="var(--app-muted)"
                          stroke-dasharray="2 2"
                        />
                      {/if}
                      {#each chartGroup.fields as field (field.name)}
                        {@const column = result?.columns[field.name]}
                        {@const paths = chartModel?.paths.get(field.name) ?? []}
                        {#if column && isFieldVisible(field.name)}
                          {#each paths as segment (segment)}
                            <path
                              d={segment}
                              fill="none"
                              stroke={field.color}
                              stroke-width="1.5"
                              vector-effect="non-scaling-stroke"
                            />
                          {/each}
                        {/if}
                      {/each}
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
                        <rect
                          x="0"
                          y="0"
                          width={selectionRange.leftX}
                          height={CHART_HEIGHT}
                          fill="var(--app-surface)"
                          fill-opacity="0.82"
                        />
                        <rect
                          x={selectionRange.rightX}
                          y="0"
                          width={CHART_WIDTH - selectionRange.rightX}
                          height={CHART_HEIGHT}
                          fill="var(--app-surface)"
                          fill-opacity="0.82"
                        />
                        <line
                          x1={selectionRange.startX}
                          x2={selectionRange.startX}
                          y1="0"
                          y2={CHART_HEIGHT}
                          stroke="var(--app-accent)"
                          stroke-width="1.5"
                        />
                        <line
                          x1={selectionRange.endX}
                          x2={selectionRange.endX}
                          y1="0"
                          y2={CHART_HEIGHT}
                          stroke="var(--app-accent)"
                          stroke-width="1.5"
                        />
                      {/if}
                      {#each tooltipEntries as entry (entry.field.name)}
                        {#if inspectionVisible}
                          <circle
                            cx={entry.point.x}
                            cy={entry.point.y}
                            r="2.75"
                            fill={entry.field.color}
                            stroke="var(--app-canvas)"
                            stroke-width="1"
                            vector-effect="non-scaling-stroke"
                          />
                        {/if}
                      {/each}
                    </svg>
                    {#if panEdge === 'start'}
                      <div
                        class="pointer-events-none absolute inset-y-0 left-0 z-20 w-1.5 rounded-l bg-warning shadow-[0_0_0.5rem_var(--app-warning)]"
                        aria-hidden="true"
                      ></div>
                    {:else if panEdge === 'end'}
                      <div
                        class="pointer-events-none absolute inset-y-0 right-0 z-20 w-1.5 rounded-r bg-warning shadow-[0_0_0.5rem_var(--app-warning)]"
                        aria-hidden="true"
                      ></div>
                    {/if}
                    {#if inspectionVisible && inspectionGroupActive}
                      <div
                        class="pointer-events-none absolute top-1 z-10 w-max max-w-none rounded border border-outline bg-surface px-2 py-1 font-mono text-[10px] leading-tight whitespace-nowrap shadow"
                        style={tooltipStyle(tooltipX)}
                        role="status"
                      >
                        {#if exactStatus === 'pending'}
                          <span>Loading exact value…</span>
                        {:else if exactStatus === 'unavailable'}
                          <span>{sessionChartField(activeField).displayName}: unavailable</span>
                        {:else}
                          <div class="grid grid-cols-[max-content_max-content] gap-x-3 gap-y-0.5">
                            {#each tooltipEntries as entry (entry.field.name)}
                              <span class="contents">
                                <span class="text-left" style={`color: ${entry.field.color}`}>
                                  {entry.field.displayName}:
                                </span>
                                <span class="text-right" style={`color: ${entry.field.color}`}>
                                  {formatTooltipValue(entry.field.name, entry.point.value)}
                                  {#if sessionFieldUnit(entry.field.name) && entry.field.format !== 'percent'}
                                    {sessionFieldUnit(entry.field.name)}
                                  {/if}
                                </span>
                              </span>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                </div>
                {#if chartGroup.fields.some( (field) => result?.columns[field.name]?.values.some((value) => value === null) )}
                  <p class="text-[10px] text-muted">Null or time gaps are shown as breaks.</p>
                {/if}
              {:else if hasColumns}<p class="mt-1 text-xs text-muted">No finite samples in this range.</p>
              {:else}<p class="mt-1 text-xs text-muted">Waiting for this field group.</p>{/if}

              {#if chartGroup.fields.length > 1}
                <div class="mt-2 flex flex-wrap gap-x-3 gap-y-1" role="group" aria-label={`${chartGroup.title} series`}>
                  {#each chartGroup.fields as field (field.name)}
                    {@const visible = isFieldVisible(field.name)}
                    <button
                      type="button"
                      class={`flex touch-manipulation items-center gap-1.5 rounded px-1 py-0.5 text-xs ${
                        visible ? 'text-foreground' : 'text-muted line-through'
                      }`}
                      aria-pressed={visible}
                      aria-label={`${visible ? 'Hide' : 'Show'} ${field.displayName} line`}
                      onpointerdown={(event) => activateFromPointer(event, () => toggleField(field.name))}
                      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, () => toggleField(field.name))}
                    >
                      <span
                        class="inline-block size-2.5 rounded-full"
                        style={`background-color: ${field.color}`}
                        aria-hidden="true"
                      ></span>
                      <span>{field.displayName}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {:else if !loading}<p class="mt-2 text-sm text-muted">No retained numeric history is available.</p>{/if}

  {#if !touchDevice && isZoomed}
    <div class="mt-2 flex flex-wrap items-center justify-between gap-2">
      <div class="flex gap-2">
        <button
          type="button"
          class="rounded-lg bg-accent px-4 py-2 text-on-accent"
          onpointerdown={(event) => activateFromPointer(event, reset)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, reset)}>Reset Zoom</button
        >
      </div>
    </div>
  {/if}
</div>

{#if touchDevice}
  <div class="fixed right-4 bottom-4 z-20 flex flex-col items-center gap-2">
    {#if isZoomed}
      <button
        type="button"
        class="flex size-14 touch-manipulation flex-col items-center justify-center rounded-full border border-outline bg-accent text-[10px] font-bold tracking-[0.12em] text-on-accent uppercase shadow-xl"
        aria-label={`Chart mode: ${interactionMode}. Switch to ${nextInteractionMode()} mode`}
        title={`Switch to ${nextInteractionMode()} mode`}
        onpointerdown={(event) => activateFromPointer(event, reset)}
        onclick={(event) => activateFromKeyboardOrAssistiveClick(event, reset)}
      >
        <span aria-hidden="true">Reset</span>
      </button>
    {/if}

    <button
      type="button"
      class="flex size-14 touch-manipulation flex-col items-center justify-center rounded-full border border-outline bg-accent text-[10px] font-bold tracking-[0.12em] text-on-accent uppercase shadow-xl"
      aria-label={`Chart mode: ${interactionMode}. Switch to ${nextInteractionMode()} mode`}
      title={`Switch to ${nextInteractionMode()} mode`}
      onpointerdown={(event) => activateFromPointer(event, cycleInteractionMode)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, cycleInteractionMode)}
    >
      <span aria-hidden="true">{interactionMode}</span>
    </button>
  </div>
{/if}
