<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';

  const units = $derived(backendState.settings?.unitPreference ?? 'metric');

  function grouped(sessionId: string) {
    return backendState.sessionGroups.some((group) => group.sessionIds.includes(sessionId));
  }
</script>

<section aria-label="Sessions" class="">
  {#if backendState.sessionGroups.length}
    <h3 class="mt-5 font-medium">Linked rides</h3>
    <ul class="mt-2 space-y-2">
      {#each backendState.sessionGroups as group (group.sessionGroupId)}
        <li class="rounded-lg bg-elevated p-3">
          <div class="flex items-start justify-between gap-3">
            <div>
              <span class="block font-medium">{group.sessionIds.length} ride segments</span>
              <span class="text-sm text-muted">
                {Math.round(group.totalElapsedMs / 60000)} min ·
                {formatDistanceM(group.distanceM, units)}
              </span>
            </div>
            <button
              class="rounded bg-surface px-2 py-1 text-xs"
              onclick={() => backendState.unlinkSessionGroup(group.sessionGroupId)}>Unlink</button
            >
          </div>
          <div class="mt-2 flex flex-wrap gap-2">
            {#each group.sessionIds as sessionId, index (`${group.sessionGroupId}-${sessionId}`)}
              {#if backendState.sessions.some((item) => item.sessionId === sessionId)}
                <a class="rounded bg-surface px-2 py-1 text-sm" href={`#/sessions/${encodeURIComponent(sessionId)}`}
                  >Segment {index + 1}</a
                >
              {/if}
            {/each}
          </div>
        </li>
      {/each}
    </ul>
  {/if}

  {#if backendState.sessions.length}
    <ul class="mt-4 space-y-2">
      {#each backendState.sessions as session (session.sessionId)}
        {#if !grouped(session.sessionId)}
          <li>
            <a
              class="block w-full rounded-lg bg-elevated p-3 text-left"
              href={`#/sessions/${encodeURIComponent(session.sessionId)}`}
            >
              <span class="block font-medium">{session.displayName}</span>
              <span class="text-sm text-muted">
                {session.state} · {Math.round(session.totalElapsedMs / 60000)} min ·
                {formatDistanceM(session.distanceM, units)}
              </span>
            </a>
          </li>
        {/if}
      {/each}
    </ul>
  {:else}
    <p class="mt-3 text-muted">Recorded sessions will appear here.</p>
  {/if}
  {#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>
