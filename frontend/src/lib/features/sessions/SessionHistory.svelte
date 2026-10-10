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
  }: {
    markers?: SessionRideMarker[];
    selectedPoint?: { elapsedMs: number } | null;
    onmarkerselect?: (marker: SessionRideMarker) => void;
    metadata: SessionMetadata;
    onselectionchange?: (elapsedMs: number) => void;
    onviewportchange?: (viewport: Viewport) => void;
  } = $props();
</script>

<section aria-labelledby="history-heading">
  <SessionHistoryChart {markers} {selectedPoint} {onmarkerselect} {metadata} {onselectionchange} {onviewportchange} />
</section>
