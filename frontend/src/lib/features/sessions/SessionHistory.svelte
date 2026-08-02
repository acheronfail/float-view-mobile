<script lang="ts">
  import type { SessionMetadata } from '$lib/backend/contract';
  import SessionHistoryChart from './SessionHistoryChart.svelte';

  let { metadata }: { metadata: SessionMetadata } = $props();
</script>

<section aria-labelledby="history-heading">
  <SessionHistoryChart {metadata} />
</section>
