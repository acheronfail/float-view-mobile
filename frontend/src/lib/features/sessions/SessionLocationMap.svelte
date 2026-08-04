<script lang="ts">
  import { onMount } from 'svelte';
  import Leaflet from 'leaflet';
  import type { SessionLocationGap, SessionLocationPoint, SessionMetadata } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import riderSvg from './rider-icon.svg?raw';

  let { metadata, selectedElapsedMs = 0 }: { metadata: SessionMetadata; selectedElapsedMs?: number } = $props();
  let node = $state<HTMLDivElement | null>(null);
  let map = $state<Leaflet.Map | null>(null);
  let baseLayers = $state<Leaflet.Polyline[]>([]);
  let travelledLayers = $state<Leaflet.Polyline[]>([]);
  let riderMarker = $state<Leaflet.Marker | null>(null);
  let points = $state<SessionLocationPoint[]>([]);
  let gaps = $state<SessionLocationGap[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

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
      if (points.length) initialiseMap();
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
    updateMap();
  }

  const segments = $derived.by(() => {
    const result: SessionLocationPoint[][] = [];
    let segment: SessionLocationPoint[] = [];
    points.forEach((point, index) => {
      const previous = points[index - 1];
      if (previous && hasGap(previous.elapsedMs, point.elapsedMs)) {
        if (segment.length) result.push(segment);
        segment = [];
      }
      segment.push(point);
    });
    if (segment.length) result.push(segment);
    return result;
  });

  const travelledSegments = $derived(
    segments
      .map((segment) => segment.filter((point) => point.elapsedMs <= selectedElapsedMs))
      .filter((segment) => segment.length)
  );

  const marker = $derived.by(() => points.filter((point) => point.elapsedMs <= selectedElapsedMs).at(-1) ?? null);

  $effect(() => {
    void points;
    void selectedElapsedMs;
    if (map) updateMap();
  });

  function hasGap(start: number, end: number): boolean {
    return end - start > 10_000 || gaps.some((gap) => start < gap.endElapsedMs && end > gap.startElapsedMs);
  }

  function updateMap() {
    if (!map || !points.length) return;
    baseLayers.forEach((layer) => layer.remove());
    travelledLayers.forEach((layer) => layer.remove());
    riderMarker?.remove();

    baseLayers = segments.map((segment) =>
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
    const bounds = Leaflet.latLngBounds(
      points.map((point) => [point.latitude, point.longitude] as Leaflet.LatLngExpression)
    );
    if (!map.getBounds().isValid() || !map.getBounds().contains(bounds)) map.fitBounds(bounds.pad(0.15));
    map.invalidateSize();
  }
</script>

<section aria-label="Session location map" class="sticky top-2 rounded-xl border border-outline bg-elevated p-3">
  <div class="mb-2 flex items-center justify-between">
    <h2 class="font-medium">Location</h2>
    {#if loading}<span class="text-xs text-muted">Loading track…</span>{/if}
  </div>
  {#if error}
    <p role="alert" class="text-sm text-warning">{error}</p>
  {:else if !points.length && !loading}
    <p class="text-sm text-muted">No accepted location points were recorded.</p>
  {:else}
    <div
      bind:this={node}
      role="img"
      aria-label="Recorded session route"
      class="h-64 w-full rounded-lg bg-surface"
    ></div>
    <p class="mt-2 text-xs text-muted">
      Accepted points: {points.length}{gaps.length ? ` · gaps: ${gaps.length}` : ''}
    </p>
  {/if}
</section>
