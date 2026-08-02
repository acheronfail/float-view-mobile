<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import ListItem from '$lib/ui/ListItem.svelte';
</script>

<section aria-label="Boards" class="flex flex-col gap-4">
  {#if backendState.savedDevices.length}
    <ul class="flex flex-col gap-2">
      {#each backendState.savedDevices as device (device.savedDeviceId)}
        <li>
          <ListItem
            href={`#/boards/${encodeURIComponent(device.savedDeviceId)}`}
            title={device.displayName}
            subtitle={device.profileComplete ? 'Configured' : 'Setup available'}
          >
            {#snippet trailing()}
              <span class="text-accent">Configure →</span>
            {/snippet}
          </ListItem>
        </li>
      {/each}
    </ul>
  {:else if backendState.status === 'connected'}
    <p class="text-muted">Boards are saved after their first connection.</p>
  {:else}
    <p class="text-muted">Loading saved boards…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="text-warning">{backendState.error}</p>{/if}
</section>
