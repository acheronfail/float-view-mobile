<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ListItem from '$lib/ui/ListItem.svelte';

  const units = $derived(backendState.settings?.unitPreference ?? 'metric');

  function grouped(sessionId: string) {
    return backendState.sessionGroups.some((group) => group.sessionIds.includes(sessionId));
  }
</script>

<section aria-label="Sessions" class="flex flex-col gap-4">
  {#if backendState.sessionGroups.length}
    <div class="flex flex-col gap-2">
      <h3 class="font-medium">Linked rides</h3>
      <ul class="flex flex-col gap-2">
        {#each backendState.sessionGroups as group (group.sessionGroupId)}
          <li class="rounded-lg bg-elevated p-3">
            <div class="flex flex-col gap-2">
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
              <div class="flex flex-wrap gap-2">
                {#each group.sessionIds as sessionId, index (`${group.sessionGroupId}-${sessionId}`)}
                  {#if backendState.sessions.some((item) => item.sessionId === sessionId)}
                    <a class="rounded bg-surface px-2 py-1 text-sm" href={`#/sessions/${encodeURIComponent(sessionId)}`}
                      >Segment {index + 1}</a
                    >
                  {/if}
                {/each}
              </div>
            </div>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if backendState.sessions.length}
    <ul class="flex flex-col gap-2">
      {#each backendState.sessions as session (session.sessionId)}
        {#if !grouped(session.sessionId)}
          <li>
            <ListItem
              href={`#/sessions/${encodeURIComponent(session.sessionId)}`}
              title={session.displayName}
              subtitle={`${session.state} · ${Math.round(session.totalElapsedMs / 60000)} min · ${formatDistanceM(session.distanceM, units)}`}
            />
          </li>
        {/if}
      {/each}
    </ul>
  {:else}
    <p class="text-muted">Recorded sessions will appear here.</p>
  {/if}
  {#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>
