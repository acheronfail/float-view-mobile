<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import { formatDistanceM } from '$lib/features/measurements/format';
  import ListItem from '$lib/ui/ListItem.svelte';

  const units = $derived(backendState.settings?.unitPreference ?? 'metric');

  function duration(milliseconds: number): string {
    const minutes = Math.round(milliseconds / 60_000);
    if (minutes < 60) return `${minutes} min riding`;
    return `${Math.floor(minutes / 60)} h ${minutes % 60} min riding`;
  }

  function summary(device: (typeof backendState.savedDevices)[number]): string {
    const value = device.summary;
    return `${value.sessionCount} ${value.sessionCount === 1 ? 'session' : 'sessions'} · ${duration(value.totalRidingElapsedMs)} · ${formatDistanceM(value.totalDistanceM, units)}`;
  }
</script>

<section aria-label="Saved boards" class="flex flex-col gap-4 py-2">
  {#if backendState.savedDevices.length}
    <ul class="flex flex-col gap-2">
      {#each backendState.savedDevices as device (device.savedDeviceId)}
        <li>
          <ListItem
            href={`#/sessions/boards/${encodeURIComponent(device.savedDeviceId)}`}
            title={device.displayName}
            subtitle={summary(device)}
          />
        </li>
      {/each}
    </ul>
  {:else if backendState.status === 'connected'}
    <p class="text-muted">Saved boards and their sessions will appear here. Guest rides are not saved or tracked.</p>
  {:else}
    <p class="text-muted">Loading saved boards…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>
