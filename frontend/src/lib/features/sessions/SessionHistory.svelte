<script lang="ts">
  import type { SessionMetadata } from '$lib/backend/contract';
  import type { Viewport } from './viewport';
  import SessionHistoryChart from './SessionHistoryChart.svelte';

  let {
    metadata,
    onselectionchange,
    onviewportchange,
  }: {
    metadata: SessionMetadata;
    onselectionchange?: (elapsedMs: number) => void;
    onviewportchange?: (viewport: Viewport) => void;
  } = $props();
</script>

<section aria-labelledby="history-heading">
  <SessionHistoryChart {metadata} {onselectionchange} {onviewportchange} />
</section>
