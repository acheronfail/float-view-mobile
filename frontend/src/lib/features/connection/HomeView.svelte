<script lang="ts">
  import { goto } from '$app/navigation';
  import { backendState } from '$lib/backend/state.svelte';
  import ListItem from '$lib/ui/ListItem.svelte';
    import SectionTitle from '../../ui/SectionTitle.svelte';
  import Spinner from '../../ui/Spinner.svelte';

  async function connect(deviceId: string) {
    if (await backendState.connectDevice(deviceId)) await goto('#/live');
  }

  function formatDeviceState(state: string) {
    return state.replaceAll('_', ' ').replace(/\b\w/g, (character) => character.toUpperCase());
  }

  // TODO: show connection strength with progress bar and dB
  // TODO: show uuid rather than just `discovered` or state or whatever that is
</script>

<section aria-label="Scan for boards" class="flex flex-col gap-4">
  {#if backendState.status === 'connected'}
    <button
      class="inline-flex items-center justify-center gap-2 rounded-lg bg-accent px-4 py-2 text-on-accent disabled:opacity-50"
      disabled={backendState.busy || backendState.scanning}
      onclick={() => backendState.scan()}
    >
      {#if backendState.scanning}
        <Spinner />
        <span aria-live="polite">Scanning…</span>
      {:else}
        Scan for boards
      {/if}
    </button>

    {#if backendState.discoveredDevices.length}
      <div class="flex flex-col gap-2">
        <SectionTitle title="Discovered boards" />
        <ul class="flex flex-col gap-2">
          {#each backendState.discoveredDevices as device (device.deviceId)}
            <li>
              <ListItem title={device.name} subtitle={formatDeviceState(device.state)}>
                {#snippet trailing()}
                  <button
                    class="rounded-md bg-accent px-3 py-1 text-on-accent disabled:opacity-50"
                    disabled={backendState.busy}
                    onclick={() => connect(device.deviceId)}>Connect</button
                  >
                {/snippet}
              </ListItem>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  {:else if backendState.status === 'failed'}
    <p role="alert" class="text-warning">{backendState.error}</p>
    <button class="rounded-lg bg-elevated px-4 py-2" onclick={() => backendState.connect()}>Retry</button>
  {:else}
    <p class="text-muted">Connecting to backend…</p>
  {/if}

  {#if backendState.error && backendState.status !== 'failed'}
    <p role="alert" class="text-warning">{backendState.error}</p>
  {/if}
</section>
