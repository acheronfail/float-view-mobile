<script lang="ts">
  import { goto } from '$app/navigation';
  import { backendState } from '$lib/backend/state.svelte';
    import Spinner from '../../ui/Spinner.svelte';

  async function connect(deviceId: string) {
    if (await backendState.connectDevice(deviceId)) await goto('#/live');
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
        <h3 class="font-medium">Nearby boards</h3>
        <ul class="space-y-2">
          {#each backendState.discoveredDevices as device (device.deviceId)}
            <li class="flex items-center justify-between gap-3 rounded-lg bg-elevated p-3">
              <div>
                <span class="block">{device.name}</span>
                <span class="text-sm text-muted capitalize">{device.state.replaceAll('_', ' ')}</span>
              </div>
              <button
                class="rounded-md bg-accent px-3 py-1 text-on-accent disabled:opacity-50"
                disabled={backendState.busy}
                onclick={() => connect(device.deviceId)}>Connect</button
              >
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
