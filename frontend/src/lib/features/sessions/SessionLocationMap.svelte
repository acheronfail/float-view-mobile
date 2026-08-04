<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Leaflet from 'leaflet';
  import type { SessionLocationGap, SessionLocationPoint, SessionMetadata } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import {
    locationAtElapsed,
    locationAtOrBefore,
    splitLocationSegments,
    visibleLocationSegments,
  } from './location-geometry';
  import type { Viewport } from './viewport';
  import riderSvg from './rider-icon.svg?raw';
  import Spinner from '../../ui/Spinner.svelte';

  let {
    metadata,
    selectedElapsedMs = 0,
    viewport = null,
    stickyTop = 0,
    height = $bindable(0),
  }: {
    metadata: SessionMetadata;
    selectedElapsedMs?: number;
    viewport?: Viewport | null;
    stickyTop?: number;
    height?: number;
  } = $props();
  let node = $state<HTMLDivElement | null>(null);
  // Leaflet owns these objects imperatively. Keeping them out of Svelte's
  // reactive graph prevents updateMap() from retriggering the effect that
  // called it whenever it replaces a layer or marker.
  let map: Leaflet.Map | null = null;
  let baseLayers: Leaflet.Polyline[] = [];
  let travelledLayers: Leaflet.Polyline[] = [];
  let riderMarker: Leaflet.Marker | null = null;
  let hasInitialView = false;
  let lastViewportKey: string | null = null;
  let mapLayoutReady = $state(false);
  let mapRefreshFrame: number | null = null;

  let points = $state<SessionLocationPoint[]>([]);
  let gaps = $state<SessionLocationGap[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let hidden = $state(false);

  onMount(() => {
    let active = true;
    if (!metadata.location.available) return;
    void loadLocations().catch((reason) => {
      if (active) error = reason instanceof Error ? reason.message : String(reason);
    });
    return () => {
      active = false;
      riderMarker?.remove();
      map?.remove();
      map = null;
      hasInitialView = false;
      lastViewportKey = null;
      mapLayoutReady = false;
      if (mapRefreshFrame !== null && typeof window !== 'undefined') {
        window.cancelAnimationFrame(mapRefreshFrame);
        mapRefreshFrame = null;
      }
      height = 0;
    };
  });

  async function loadLocations() {
    loading = true;
    try {
      const controller = backendState.createSessionReviewController(metadata);
      let cursor: string | null = null;
      const loaded: SessionLocationPoint[] = [];
      const loadedGaps: SessionLocationGap[] = [];
      for (let page = 0; page < 4; page += 1) {
        const result = await controller.loadLocations({
          kind: 'locations',
          range: { startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs },
          limit: 2_000,
          cursor,
        });
        loaded.push(...result.points);
        loadedGaps.push(...result.gaps);
        cursor = result.nextCursor;
        if (!cursor) break;
      }
      points = loaded;
      gaps = loadedGaps;
      if (points.length) {
        await tick();
        initialiseMap();
      }
    } finally {
      loading = false;
    }
  }

  function initialiseMap() {
    if (!node || map || !points.length) return;
    map = Leaflet.map(node, { preferCanvas: true, zoomControl: true });
    Leaflet.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
      maxZoom: 19,
      attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
    }).addTo(map);
    if (hidden) {
      lastViewportKey = viewportKey;
      return;
    }
    mapLayoutReady = true;
    updateMap(true);
    lastViewportKey = viewportKey;
  }

  function observeSize(element: HTMLElement) {
    const update = () => {
      height = Math.ceil(element.getBoundingClientRect().height);
    };
    update();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return { destroy: () => observer.disconnect() };
  }

  const segments = $derived(splitLocationSegments(points, gaps));
  const isZoomed = $derived(
    viewport !== null && viewport.endElapsedMs - viewport.startElapsedMs < metadata.summary.totalElapsedMs
  );
  const visibleSegments = $derived.by(() => {
    if (!isZoomed || !viewport) return segments;
    return visibleLocationSegments(points, gaps, viewport.startElapsedMs, viewport.endElapsedMs);
  });
  const visiblePoints = $derived(visibleSegments.flat());
  const travelledSegments = $derived(
    visibleSegments
      .map((segment) => segment.filter((point) => point.elapsedMs <= selectedElapsedMs))
      .filter((segment) => segment.length > 0)
  );
  const marker = $derived.by(() => {
    if (!isZoomed || !viewport) return locationAtOrBefore(points, selectedElapsedMs);
    if (!visiblePoints.length) return null;
    if (selectedElapsedMs >= viewport.startElapsedMs && selectedElapsedMs <= viewport.endElapsedMs) {
      return locationAtOrBefore(visiblePoints, selectedElapsedMs) ?? visiblePoints[0]!;
    }
    return locationAtElapsed(visibleSegments, (viewport.startElapsedMs + viewport.endElapsedMs) / 2);
  });
  const viewportKey = $derived.by(() => {
    if (!isZoomed || !viewport) return 'full';
    return `${viewport.startElapsedMs}:${viewport.endElapsedMs}`;
  });

  $effect(() => {
    void points;
    void selectedElapsedMs;
    void viewportKey;
    void hidden;
    void mapLayoutReady;
    if (!map || hidden || !mapLayoutReady) return;
    const shouldRecenter = lastViewportKey !== viewportKey;
    updateMap(shouldRecenter);
    lastViewportKey = viewportKey;
  });

  function updateMap(shouldRecenter = false) {
    if (!map) return;
    baseLayers.forEach((layer) => layer.remove());
    travelledLayers.forEach((layer) => layer.remove());
    riderMarker?.remove();

    baseLayers = visibleSegments.map((segment) =>
      Leaflet.polyline(
        segment.map((point) => [point.latitude, point.longitude] as Leaflet.LatLngExpression),
        {
          color: '#67e8f9',
          weight: 3,
          opacity: 0.55,
          dashArray: '5 5',
          interactive: false,
        }
      ).addTo(map!)
    );
    travelledLayers = travelledSegments.map((segment) =>
      Leaflet.polyline(
        segment.map((point) => [point.latitude, point.longitude] as Leaflet.LatLngExpression),
        {
          color: '#38bdf8',
          weight: 4,
          interactive: false,
        }
      ).addTo(map!)
    );
    if (marker) {
      riderMarker = Leaflet.marker([marker.latitude, marker.longitude], {
        icon: Leaflet.divIcon({ className: 'rider-icon', html: riderSvg, iconSize: [32, 48], iconAnchor: [16, 40] }),
        title: 'Selected location',
      }).addTo(map);
    }
    fitVisibleRoute(shouldRecenter);
    map.invalidateSize();
  }

  function fitVisibleRoute(force = false) {
    if (!map || !visiblePoints.length) return;
    const bounds = Leaflet.latLngBounds(
      visiblePoints.map((point) => [point.latitude, point.longitude] as Leaflet.LatLngExpression)
    );
    if (force || !hasInitialView || !map.getBounds().contains(bounds)) {
      map.fitBounds(bounds.pad(0.15));
      hasInitialView = true;
    }
  }

  function recenterMap() {
    fitVisibleRoute(true);
    map?.invalidateSize();
  }

  function toggleMap() {
    hidden = !hidden;
    mapLayoutReady = false;
    lastViewportKey = null;
    if (!hidden) scheduleMapRefresh();
  }

  function scheduleMapRefresh() {
    if (!map || typeof window === 'undefined') return;
    if (mapRefreshFrame !== null) window.cancelAnimationFrame(mapRefreshFrame);
    mapRefreshFrame = window.requestAnimationFrame(() => {
      mapRefreshFrame = window.requestAnimationFrame(() => {
        mapRefreshFrame = null;
        if (hidden || !map) return;
        map.invalidateSize({ pan: false });
        mapLayoutReady = true;
      });
    });
  }
</script>

<section
  use:observeSize
  aria-label="Session location map"
  class="sticky z-11 bg-canvas"
  style={`top: calc(max(1.5rem, env(safe-area-inset-top, 0px)) + ${stickyTop}px)`}
>
  <div class="mb-2 flex items-center justify-between">
    {#if loading}
      <span class="flex flex-row items-center gap-2 text-xs text-muted">
        <Spinner />
        Loading track…
      </span>
    {/if}
  </div>

  {#if error}
    <p role="alert" class="text-sm text-warning">{error}</p>
  {:else if !points.length && !loading}
    <p class="text-sm text-muted">No accepted location points were recorded.</p>
  {:else}
    <div class="flex flex-col gap-1">
      <div
        bind:this={node}
        role="img"
        aria-label="Recorded session route"
        class="h-64 w-full rounded-lg bg-surface"
        class:hidden
      ></div>
      <div class="flex flex-row items-center justify-between">
        {#if hidden}
          <span></span>
        {:else}
          <button
            type="button"
            class="touch-manipulation rounded bg-accent px-2 text-sm text-on-accent"
            aria-label="Re-center route map"
            onpointerdown={(event) => activateFromPointer(event, recenterMap)}
            onclick={(event) => activateFromKeyboardOrAssistiveClick(event, recenterMap)}
          >
            re-center
          </button>
        {/if}
        <button
          type="button"
          class="touch-manipulation rounded bg-accent px-2 text-sm text-on-accent"
          aria-label="Show/hide the map"
          onpointerdown={(event) => activateFromPointer(event, toggleMap)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, toggleMap)}
        >
          {#if hidden}
            Show Map
          {:else}
            Hide Map
          {/if}
        </button>
      </div>
    </div>
  {/if}
</section>
