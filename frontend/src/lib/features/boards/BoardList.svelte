<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
</script>

<section aria-label="Boards">
  {#if backendState.savedDevices.length}
    <ul class="mt-4 space-y-2">
      {#each backendState.savedDevices as device (device.savedDeviceId)}
        <li>
          <a
            class="flex items-center justify-between gap-3 rounded-lg bg-elevated p-3"
            href={`#/boards/${encodeURIComponent(device.savedDeviceId)}`}
          >
            <div>
              <span class="block font-medium">{device.displayName}</span>
              <span class="text-sm text-muted">
                {device.profileComplete ? 'Configured' : 'Setup available'}
              </span>
            </div>
            <span class="text-accent">Configure →</span>
          </a>
        </li>
      {/each}
    </ul>
  {:else if backendState.status === 'connected'}
    <p class="mt-3 text-muted">Boards are saved after their first connection.</p>
  {:else}
    <p class="mt-3 text-muted">Loading saved boards…</p>
  {/if}
  {#if backendState.error}<p role="alert" class="mt-3 text-warning">{backendState.error}</p>{/if}
</section>
