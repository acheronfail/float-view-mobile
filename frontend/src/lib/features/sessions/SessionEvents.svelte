<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import type { SessionMetadata } from '$lib/backend/contract';
  import type { SessionReviewController } from '$lib/backend/session-controller';
  import { backendState } from '$lib/backend/state.svelte';

  let { metadata }: { metadata: SessionMetadata } = $props();
  let controller: SessionReviewController | null = null;
  let cursor = $state<string | null>(null);
  let rows = $state<{ sequence: number; elapsedMs: number; kind: string; value: string }[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function loadMore() {
    if (loading || (rows.length && !cursor)) return;
    loading = true;
    error = null;
    try {
      controller ??= backendState.createSessionReviewController(metadata);
      const page = await controller.loadEvents({
        kind: 'events',
        range: { startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs },
        limit: 100,
        cursor,
      });
      const known = new Set(rows.map((row) => row.sequence));
      rows = [
        ...rows,
        ...page.page.sequence
          .map((sequence, index) => ({
            sequence,
            elapsedMs: page.page.elapsedMs[index]!,
            kind: page.page.kinds[index]!,
            value: page.page.values[index]!,
          }))
          .filter((row) => !known.has(row.sequence)),
      ];
      cursor = page.nextCursor;
    } catch (reason) {
      if (!(reason instanceof DOMException && reason.name === 'AbortError')) {
        error = reason instanceof Error ? reason.message : String(reason);
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => void loadMore());
  onDestroy(() => controller?.dispose());
</script>

<section class="mt-5 rounded-lg border border-outline p-4" aria-labelledby="events-heading">
  <h3 id="events-heading" class="font-medium">State transitions</h3>
  {#if error}<p class="mt-2 text-sm text-warning" role="alert">{error}</p>{/if}
  {#if rows.length}
    <ol class="mt-2 space-y-1 text-sm">
      {#each rows as row (row.sequence)}
        <li><time>{row.elapsedMs} ms</time> · {row.kind}: {row.value}</li>
      {/each}
    </ol>
  {:else if !loading}<p class="mt-2 text-sm text-muted">No transitions are available for this snapshot.</p>{/if}
  {#if loading}<p class="mt-2 text-xs text-muted" role="status">Loading transitions…</p>{/if}
  {#if cursor}
    <button type="button" class="mt-3 rounded bg-elevated px-3 py-2 text-xs" onclick={loadMore}>Load more</button>
  {/if}
</section>
