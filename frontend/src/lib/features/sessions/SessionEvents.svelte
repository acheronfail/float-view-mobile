<script lang="ts">
  import { untrack } from 'svelte';
  import { activateFromPointer, activateFromKeyboardOrAssistiveClick } from '$lib/ui/activation';
  import type { Viewport } from './viewport';
  import type { SessionMetadata } from '$lib/backend/contract';
  import type { SessionReviewController } from '$lib/backend/session-controller';
  import { backendState } from '$lib/backend/state.svelte';

  let { metadata, viewport }: { metadata: SessionMetadata; viewport: Viewport | null } = $props();
  let expanded = $state(false);
  let context: { controller: SessionReviewController; range: Viewport; active: boolean } | null = null;
  const range = $derived(viewport ?? { startElapsedMs: 0, endElapsedMs: metadata.summary.totalElapsedMs });
  let cursor = $state<string | null>(null);
  let rows = $state<{ sequence: number; elapsedMs: number; kind: string; value: string }[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function loadMore() {
    const request = context;
    if (!request?.active || loading || (rows.length && !cursor)) return;
    loading = true;
    error = null;
    try {
      const page = await request.controller.loadEvents({
        kind: 'events',
        range: request.range,
        limit: 100,
        cursor,
      });
      if (!request.active) return;
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
      if (request.active && !(reason instanceof DOMException && reason.name === 'AbortError')) {
        error = reason instanceof Error ? reason.message : String(reason);
      }
    } finally {
      if (request.active) loading = false;
    }
  }

  $effect(() => {
    const currentMetadata = metadata;
    const currentRange = range;
    rows = [];
    cursor = null;
    error = null;
    loading = false;
    if (!expanded) return;
    const request = {
      controller: backendState.createSessionReviewController(currentMetadata),
      range: currentRange,
      active: true,
    };
    context = request;
    untrack(() => void loadMore());
    return () => {
      request.active = false;
      request.controller.dispose();
      context = null;
    };
  });

  function toggle() {
    expanded = !expanded;
  }
</script>

<section class="mt-5 rounded-lg border border-outline p-4" aria-labelledby="events-heading">
  <h3 id="events-heading" class="font-medium">
    <button
      type="button"
      class="flex min-h-11 w-full touch-manipulation items-center justify-between text-left"
      aria-expanded={expanded}
      aria-controls="session-events-content"
      onpointerdown={(event) => activateFromPointer(event, toggle)}
      onclick={(event) => activateFromKeyboardOrAssistiveClick(event, toggle)}
      >State transitions <span aria-hidden="true">{expanded ? '−' : '+'}</span></button
    >
  </h3>
  <div id="session-events-content" hidden={!expanded}>
    {#if expanded}
      <p class="mt-2 text-xs text-muted">Current chart range: {range.startElapsedMs}–{range.endElapsedMs} ms</p>
      {#if error}<p class="mt-2 text-sm text-warning" role="alert">{error}</p>{/if}
      {#if rows.length}
        <ol class="mt-2 space-y-1 text-sm">
          {#each rows as row (row.sequence)}
            <li><time>{row.elapsedMs} ms</time> · {row.kind}: {row.value}</li>
          {/each}
        </ol>
      {:else if !loading && !error}<p class="mt-2 text-sm text-muted">
          No transitions are available in this chart range.
        </p>{/if}
      {#if loading}<p class="mt-2 text-xs text-muted" role="status">Loading transitions…</p>{/if}
      {#if cursor || error}
        <button
          type="button"
          class="mt-3 min-h-11 touch-manipulation rounded bg-elevated px-3 py-2 text-xs"
          disabled={loading}
          onpointerdown={(event) => activateFromPointer(event, loadMore)}
          onclick={(event) => activateFromKeyboardOrAssistiveClick(event, loadMore)}
          >{error ? 'Retry' : 'Load more'}</button
        >
      {/if}
    {/if}
  </div>
</section>
