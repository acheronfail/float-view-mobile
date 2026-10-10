<script lang="ts">
  import type { SessionMetadata, SessionRideMarker } from '$lib/backend/contract';
  import type { Viewport } from './viewport';
  import SessionHistoryChart from './SessionHistoryChart.svelte';

  let {
    markers = [],
    selectedPoint = null,
    onmarkerselect = () => {},
    metadata,
    onselectionchange,
    onviewportchange,
    filterByMap = false,
    intervals = null,
    filterError = null,
    ondataerror,
    initialViewport = null,
  }: {
    markers?: SessionRideMarker[];
    selectedPoint?: { elapsedMs: number } | null;
    onmarkerselect?: (marker: SessionRideMarker) => void;
    metadata: SessionMetadata;
    onselectionchange?: (elapsedMs: number) => void;
    onviewportchange?: (viewport: Viewport) => void;
    filterByMap?: boolean;
    intervals?: Viewport[] | null;
    filterError?: string | null;
    ondataerror?: (reason: unknown) => void;
    initialViewport?: Viewport | null;
  } = $props();
</script>

<section aria-labelledby="history-heading">
  <SessionHistoryChart
    {markers}
    {selectedPoint}
    {onmarkerselect}
    {metadata}
    {onselectionchange}
    {onviewportchange}
    {filterByMap}
    {intervals}
    {filterError}
    {ondataerror}
    {initialViewport}
  />
</section>
