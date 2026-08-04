<script lang="ts">
  import type { SessionMetadata } from '$lib/backend/contract';
  import SessionHistoryChart from './SessionHistoryChart.svelte';

  let { metadata, onselectionchange }: { metadata: SessionMetadata; onselectionchange?: (elapsedMs: number) => void } =
    $props();
</script>

<section aria-labelledby="history-heading">
  <SessionHistoryChart {metadata} {onselectionchange} />
</section>
