<script lang="ts">
  import type { SessionMetadata } from '$lib/backend/contract';
  import SessionHistoryChart from './SessionHistoryChart.svelte';

  let { metadata }: { metadata: SessionMetadata } = $props();
</script>

<section class="relative mt-5 rounded-lg border border-outline p-4" aria-labelledby="history-heading">
  <h3 id="history-heading" class="font-medium">Charts</h3>
  <SessionHistoryChart {metadata} />
</section>
