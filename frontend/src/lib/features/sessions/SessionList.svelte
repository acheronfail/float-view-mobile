<script lang="ts">
  import type { SessionSummary } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ListItem from '$lib/ui/ListItem.svelte';
  import SectionTitle from '$lib/ui/SectionTitle.svelte';

  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  const dateFormatter = new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'long', day: 'numeric' });

  type SessionDateGroup = {
    key: string;
    label: string;
    sessions: SessionSummary[];
  };

  function sessionDate(startedAt: string): Date | null {
    const date = new Date(startedAt);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  function sessionDateKey(startedAt: string): string {
    const date = sessionDate(startedAt);
    if (!date) return 'unknown';

    return [date.getFullYear(), date.getMonth() + 1, date.getDate()]
      .map((value) => String(value).padStart(2, '0'))
      .join('-');
  }

  const sessionsByDate = $derived.by(() => {
    const linkedSessionIds = new Set(backendState.sessionGroups.flatMap((group) => group.sessionIds));
    const groups: SessionDateGroup[] = [];

    for (const session of backendState.sessions) {
      if (linkedSessionIds.has(session.sessionId)) continue;

      const key = sessionDateKey(session.startedAt);
      const existing = groups.find((group) => group.key === key);
      if (existing) {
        existing.sessions.push(session);
        continue;
      }

      const date = sessionDate(session.startedAt);
      groups.push({
        key,
        label: date ? dateFormatter.format(date) : 'Unknown date',
        sessions: [session],
      });
    }

    return groups.sort((left, right) => {
      if (left.key === 'unknown') return 1;
      if (right.key === 'unknown') return -1;
      return right.key.localeCompare(left.key);
    });
  });
</script>

<section aria-label="Sessions" class="flex flex-col gap-4">
  {#if backendState.sessionGroups.length}
    <div class="flex flex-col gap-2">
      <SectionTitle title="Linked rides" />
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
    {#each sessionsByDate as dateGroup (dateGroup.key)}
      <div class="flex flex-col gap-2">
        <SectionTitle title={dateGroup.label} />
        <ul class="flex flex-col gap-2">
          {#each dateGroup.sessions as session (session.sessionId)}
            <li>
              <ListItem
                href={`#/sessions/${encodeURIComponent(session.sessionId)}`}
                title={session.displayName}
                subtitle={`${session.state} · ${Math.round(session.totalElapsedMs / 60000)} min · ${formatDistanceM(session.distanceM, units)}`}
              />
            </li>
          {/each}
        </ul>
      </div>
    {/each}
    {#if !sessionsByDate.length && !backendState.sessionGroups.length}
      <p class="text-muted">Recorded sessions will appear here.</p>
    {/if}
  {:else if !backendState.sessionGroups.length}
    <p class="text-muted">Recorded sessions will appear here.</p>
  {/if}
  {#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>
