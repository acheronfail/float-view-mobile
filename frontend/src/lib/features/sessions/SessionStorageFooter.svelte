<script lang="ts">
  import { backendState } from '$lib/backend/state.svelte';
  import PageFooter from '$lib/ui/PageFooter.svelte';
  import { formatStorageBytes, totalSessionStorageBytes } from './storage';

  let { savedDeviceId }: { savedDeviceId?: string } = $props();
  const bytes = $derived(
    savedDeviceId === undefined
      ? totalSessionStorageBytes(backendState.savedDevices)
      : (backendState.savedDevices.find((device) => device.savedDeviceId === savedDeviceId)?.summary
          .totalStorageBytes ?? null)
  );
</script>

<PageFooter label="Session storage">
  <p>
    {#if !backendState.snapshot}
      Loading session storage…
    {:else if bytes === null}
      Session storage unavailable
    {:else}
      Sessions on disk · {formatStorageBytes(bytes)}
    {/if}
  </p>
</PageFooter>
