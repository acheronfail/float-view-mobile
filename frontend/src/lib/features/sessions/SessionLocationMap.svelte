<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Leaflet from 'leaflet';
  import type {
    SessionLocationGap,
    SessionLocationPoint,
    SessionMetadata,
    SessionRideMarker,
  } from '$lib/backend/contract';
  import type { SessionReviewController } from '$lib/backend/session-controller';
  import { backendState } from '$lib/backend/state.svelte';
  import { activateFromKeyboardOrAssistiveClick, activateFromPointer } from '$lib/ui/activation';
  import { iconClasses } from '$lib/ui/icon';
  import {
    locationAtElapsed,
    locationAtOrBefore,
    splitLocationSegments,
    visibleLocationSegments,
  } from './location-geometry';
  import type { Viewport } from './viewport';
  import Spinner from '../../ui/Spinner.svelte';

  const locationMarkerHtml = `<span class="${iconClasses['map-pin']}" aria-hidden="true"></span>`;

  let {
    markers = [],
    selectedMarker = null,
    onmarkerselect = () => {},
    metadata,
    selectedElapsedMs = 0,
    viewport = null,
    height = $bindable(0),
  }: {
    markers?: SessionRideMarker[];
    selectedMarker?: SessionRideMarker | null;
    onmarkerselect?: (marker: SessionRideMarker) => void;
    metadata: SessionMetadata;
    selectedElapsedMs?: number;
    viewport?: Viewport | null;
    height?: number;
  } = $props();
  let node = $state<HTMLDivElement | null>(null);
  // Leaflet owns these objects imperatively. Keeping them out of Svelte's
  // reactive graph prevents updateMap() from retriggering the effect that
  // called it whenever it replaces a layer or marker.
  let map: Leaflet.Map | null = null;
  let baseLayers: Leaflet.Polyline[] = [];
  let travelledLayers: Leaflet.Polyline[] = [];
  let locationMarker: Leaflet.Marker | null = null;
  let rideLayers: Leaflet.Marker[] = [];
  const locatedMarkers = $derived(markers.filter((m) => m.location !== null));
  let hasInitialView = false;
  let lastViewportKey: string | null = null;
  let mapLayoutReady = $state(false);
  let mapRefreshFrame: number | null = null;

  let points = $state<SessionLocationPoint[]>([]);
  let gaps = $state<SessionLocationGap[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let hidden = $state(false);

  let active = false;
  let controller: SessionReviewController | null = null;
  onMount(() => {
    active = true;

    void loadLocations().catch((reason) => {
      if (active) error = reason instanceof Error ? reason.message : String(reason);
    });
    return () => {
      active = false;
      controller?.dispose();
      rideLayers.forEach((layer) => layer.remove());
      locationMarker?.remove();
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
      controller = backendState.createSessionReviewController(metadata);
      let cursor: string | null = null;
      const loaded: SessionLocationPoint[] = [];
      const loadedGaps: SessionLocationGap[] = [];
      for (let page = 0; page < 4 && metadata.location.available; page += 1) {
        const result = await controller.loadLocations({
          kind: 'locations',
          range: { startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs },
          limit: 2_000,
          cursor,
        });
        if (!active) return;
        loaded.push(...result.points);
        loadedGaps.push(...result.gaps);
        cursor = result.nextCursor;
        if (!cursor) break;
      }
      points = loaded;
      gaps = loadedGaps;
      if (points.length || locatedMarkers.length) {
        await tick();
        if (active) initialiseMap();
      }
    } finally {
      if (active) loading = false;
    }
  }

  function initialiseMap() {
    if (!node || map || (!points.length && !locatedMarkers.length)) return;
    map = Leaflet.map(node, { preferCanvas: true, zoomControl: true });
    map.on('zoomend', drawRideMarkers);
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
    void markers;
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
    locationMarker?.remove();

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
      locationMarker = Leaflet.marker([marker.latitude, marker.longitude], {
        icon: Leaflet.divIcon({
          className: 'location-marker',
          html: locationMarkerHtml,
          iconSize: [32, 48],
          iconAnchor: [16, 40],
        }),
        title: 'Selected location',
      }).addTo(map);
    }
    fitVisibleRoute(shouldRecenter);
    drawRideMarkers();
    map.invalidateSize();
  }

  function fitVisibleRoute(force = false) {
    if (!map || (!visiblePoints.length && !locatedMarkers.length)) return;
    const bounds = Leaflet.latLngBounds(
      [...visiblePoints, ...locatedMarkers.map((m) => m.location!)].map(
        (point) => [point.latitude, point.longitude] as Leaflet.LatLngExpression
      )
    );
    if (force || !hasInitialView || !map.getBounds().contains(bounds)) {
      map.fitBounds(bounds.pad(0.15));
      hasInitialView = true;
    }
  }

  $effect(() => {
    const location = selectedMarker?.location;
    if (mapLayoutReady && map && location)
      map.setView([location.latitude, location.longitude], map.getZoom(), { animate: false });
  });

  function drawRideMarkers() {
    if (!map) return;
    rideLayers.forEach((layer) => layer.remove());
    rideLayers = [];
    // Ephemeral grouping for this Leaflet redraw, not reactive component state.
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    const groups = new Map<string, SessionRideMarker[]>();
    for (const marker of locatedMarkers) {
      const p = map.latLngToLayerPoint([marker.location!.latitude, marker.location!.longitude]);
      const key = `${Math.round(p.x / 24)}:${Math.round(p.y / 24)}`;
      groups.set(key, [...(groups.get(key) ?? []), marker]);
    }
    for (const group of groups.values()) {
      const marker = group[0]!;
      const label =
        group.length > 1
          ? `${group.length} ride markers; use marker navigator to select each`
          : `Ride marker at ${(marker.elapsedMs / 1000).toFixed(1)} seconds`;
      const layer = Leaflet.marker([marker.location!.latitude, marker.location!.longitude], {
        icon: Leaflet.divIcon({
          className: 'ride-marker',
          html: `<span style="display:grid;place-items:center;width:28px;height:28px;border:2px solid #111;border-radius:6px;background:#f97316;color:#111;box-shadow:0 0 0 2px white;font-weight:700">${group.length > 1 ? group.length : '⚑'}</span>`,
          iconSize: [32, 32],
          iconAnchor: [16, 16],
        }),
        title: label,
        alt: label,
        keyboard: true,
      }).addTo(map);
      const node = layer.getElement();
      if (node) {
        node.setAttribute('role', 'button');
        node.setAttribute('aria-label', label);
        node.style.touchAction = 'manipulation';
        node.addEventListener('pointerdown', (event) => {
          event.stopPropagation();
          activateFromPointer(event, () => onmarkerselect(marker));
        });
        node.addEventListener('click', (event) => {
          if (event.detail === 0) onmarkerselect(marker);
        });
        node.addEventListener('keydown', (event) => {
          if (event.key === ' ' || event.key === 'Enter') {
            event.preventDefault();
            if (!event.repeat) onmarkerselect(marker);
          }
        });
      }
      rideLayers.push(layer);
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
  style="top: var(--app-header-bottom, 0px)"
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
  {:else if !points.length && !locatedMarkers.length && !loading}
    <p class="text-sm text-muted">No accepted location points were recorded.</p>
  {:else}
    <div class="flex flex-col gap-1">
      <div
        bind:this={node}
        role="img"
        aria-label="Recorded session route"
        class="h-64 w-full rounded-lg"
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
