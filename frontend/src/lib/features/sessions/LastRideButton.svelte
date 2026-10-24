<script lang="ts">
  import type { SessionSummary } from '$lib/backend/contract';
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ListItem from '$lib/ui/ListItem.svelte';
  import { latestRideBoard } from './last-ride';
  import { sessionStartDateLabel } from './session-label';

  const board = $derived(latestRideBoard(backendState.savedDevices));
  const savedDeviceId = $derived(board?.savedDeviceId);
  // Snapshot objects are replaced even when the session catalog is unchanged.
  const catalogRevision = $derived(backendState.sessionCatalogRevision);
  const units = $derived(backendState.settings?.unitPreference ?? 'metric');
  let loaded = $state<{ savedDeviceId: string; session: SessionSummary | null } | null>(null);
  const session = $derived(loaded?.savedDeviceId === savedDeviceId ? loaded?.session : null);
  let error = $state<string | null>(null);
  let retry = $state(0);

  $effect(() => {
    const id = savedDeviceId;
    const revision = catalogRevision;
    void retry;
    error = null;
    if (!id) {
      loaded = null;
      return;
    }
    let cancelled = false;
    void backendState.listSessions(id, 1).then(
      (page) => {
        if (!cancelled && page.catalogRevision === revision) {
          loaded = { savedDeviceId: id, session: page.items[0] ?? null };
        }
      },
      (cause) => {
        if (!cancelled) error = cause instanceof Error ? cause.message : String(cause);
      }
    );
    return () => {
      cancelled = true;
    };
  });
</script>

{#if session && board}
  <ListItem
    href={`#/sessions/${encodeURIComponent(session.sessionId)}?board=${encodeURIComponent(board.savedDeviceId)}`}
    title="Most recent ride"
    subtitle={`${board.displayName} · ${sessionStartDateLabel(session.startedAt) ?? 'Unknown date'} · ${Math.round(session.totalElapsedMs / 60000)} min · ${formatDistanceM(session.distanceM, units)}`}
  />
{/if}
{#if error}
  <div>
    <p role="alert" class="text-warning">Unable to load most recent ride: {error}</p>
    <ListItem
      title="Try again"
      subtitle="Load most recent ride"
      onactivate={() => {
        retry += 1;
      }}
    />
  </div>
{/if}
